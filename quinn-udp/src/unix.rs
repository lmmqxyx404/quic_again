use std::{
    io::{self, IoSliceMut},
    mem,
    net::IpAddr,
    os::fd::AsRawFd,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use socket2::SockRef;

use super::log::debug;

use crate::{cmsg, RecvMeta, Transmit, UdpSockRef};

#[cfg(not(any(target_os = "freebsd", target_os = "netbsd")))]
type IpTosTy = libc::c_int;

/// Tokio-compatible UDP socket with some useful specializations.
///
/// Unlike a standard tokio UDP socket, this allows ECN bits to be read and written on some
/// platforms.
#[derive(Debug)]
pub struct UdpSocketState {
    /// 1.
    may_fragment: bool,
    /// 2.
    gro_segments: usize,
    /// 3. True if we have received EINVAL error from `sendmsg` system call at least once.
    ///
    /// If enabled, we assume that old kernel is used and switch to fallback mode.
    /// In particular, we do not use IP_TOS cmsg_type in this case,
    /// which is not supported on Linux <3.13 and results in not sending the UDP packet at all.
    sendmsg_einval: AtomicBool,
}

impl UdpSocketState {
    pub fn new(sock: UdpSockRef<'_>) -> io::Result<Self> {
        let io = sock.0;
        let mut cmsg_platform_space = 0;
        if cfg!(target_os = "linux")
            || cfg!(target_os = "freebsd")
            || cfg!(target_os = "openbsd")
            || cfg!(target_os = "netbsd")
            || cfg!(target_os = "macos")
            || cfg!(target_os = "ios")
            || cfg!(target_os = "android")
        {
            cmsg_platform_space +=
                unsafe { libc::CMSG_SPACE(mem::size_of::<libc::in6_pktinfo>() as _) as usize };
        }

        assert!(
            CMSG_LEN
                >= unsafe { libc::CMSG_SPACE(mem::size_of::<libc::c_int>() as _) as usize }
                    + cmsg_platform_space
        );

        assert!(
            mem::align_of::<libc::cmsghdr>() <= mem::align_of::<cmsg::Aligned<[u8; 0]>>(),
            "control message buffers will be misaligned"
        );

        io.set_nonblocking(true)?;

        let addr = io.local_addr()?;
        let is_ipv4 = addr.family() == libc::AF_INET as libc::sa_family_t;

        // mac and ios do not support IP_RECVTOS on dual-stack sockets :(
        // older macos versions also don't have the flag and will error out if we don't ignore it
        #[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
        if is_ipv4 || !io.only_v6()? {
            if let Err(_err) =
                set_socket_option(&*io, libc::IPPROTO_IP, libc::IP_RECVTOS, OPTION_ON)
            {
                debug!("Ignoring error setting IP_RECVTOS on socket: {_err:?}");
            }
        }

        let mut may_fragment = false;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            // opportunistically try to enable GRO. See gro::gro_segments().
            #[cfg(target_os = "linux")]
            let _ = set_socket_option(&*io, libc::SOL_UDP, libc::UDP_GRO, OPTION_ON);

            // Forbid IPv4 fragmentation. Set even for IPv6 to account for IPv6 mapped IPv4 addresses.
            // Set `may_fragment` to `true` if this option is not supported on the platform.
            may_fragment |= !set_socket_option_supported(
                &*io,
                libc::IPPROTO_IP,
                libc::IP_MTU_DISCOVER,
                libc::IP_PMTUDISC_PROBE,
            )?;

            if is_ipv4 {
                set_socket_option(&*io, libc::IPPROTO_IP, libc::IP_PKTINFO, OPTION_ON)?;
            } else {
                // Set `may_fragment` to `true` if this option is not supported on the platform.
                may_fragment |= !set_socket_option_supported(
                    &*io,
                    libc::IPPROTO_IPV6,
                    libc::IPV6_MTU_DISCOVER,
                    libc::IP_PMTUDISC_PROBE,
                )?;
            }
        }
        #[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "ios"))]
        {
            todo!()
        }
        #[cfg(any(
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "macos",
            target_os = "ios"
        ))]
        // IP_RECVDSTADDR == IP_SENDSRCADDR on FreeBSD
        // macOS uses only IP_RECVDSTADDR, no IP_SENDSRCADDR on macOS
        // macOS also supports IP_PKTINFO
        {
            todo!()
        }

        // Options standardized in RFC 3542
        if !is_ipv4 {
            set_socket_option(&*io, libc::IPPROTO_IPV6, libc::IPV6_RECVPKTINFO, OPTION_ON)?;
            set_socket_option(&*io, libc::IPPROTO_IPV6, libc::IPV6_RECVTCLASS, OPTION_ON)?;
            // Linux's IP_PMTUDISC_PROBE allows us to operate under interface MTU rather than the
            // kernel's path MTU guess, but actually disabling fragmentation requires this too. See
            // __ip6_append_data in ip6_output.c.
            // Set `may_fragment` to `true` if this option is not supported on the platform.
            may_fragment |=
                !set_socket_option_supported(&*io, libc::IPPROTO_IPV6, IPV6_DONTFRAG, OPTION_ON)?;
        }

        let now = Instant::now();
        Ok(Self {
            may_fragment,
            gro_segments: gro::gro_segments(),
            sendmsg_einval: AtomicBool::new(false),
        })
    }
    /// 2. Whether transmitted datagrams might get fragmented by the IP layer
    ///
    /// Returns `false` on targets which employ e.g. the `IPV6_DONTFRAG` socket option.
    #[inline]
    pub fn may_fragment(&self) -> bool {
        self.may_fragment
    }
    /// 3. The number of segments to read when GRO is enabled. Used as a factor to
    /// compute the receive buffer size.
    ///
    /// Returns 1 if the platform doesn't support GRO.
    #[inline]
    pub fn gro_segments(&self) -> usize {
        self.gro_segments
    }

    pub fn recv(
        &self,
        socket: UdpSockRef<'_>,
        bufs: &mut [IoSliceMut<'_>],
        meta: &mut [RecvMeta],
    ) -> io::Result<usize> {
        recv(socket.0, bufs, meta)
    }

    pub fn send(&self, socket: UdpSockRef<'_>, transmit: &Transmit<'_>) -> io::Result<()> {
        send(self, socket.0, transmit)
    }

    /// Returns true if we previously got an EINVAL error from `sendmsg` syscall.
    fn sendmsg_einval(&self) -> bool {
        self.sendmsg_einval.load(Ordering::Relaxed)
    }
}

const CMSG_LEN: usize = 88;

fn set_socket_option(
    socket: &impl AsRawFd,
    level: libc::c_int,
    name: libc::c_int,
    value: libc::c_int,
) -> io::Result<()> {
    let rc = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            level,
            name,
            &value as *const _ as _,
            mem::size_of_val(&value) as _,
        )
    };

    match rc == 0 {
        true => Ok(()),
        false => Err(io::Error::last_os_error()),
    }
}

const OPTION_ON: libc::c_int = 1;

/// Returns whether the given socket option is supported on the current platform
///
/// Yields `Ok(true)` if the option was set successfully, `Ok(false)` if setting
/// the option raised an `ENOPROTOOPT` error, and `Err` for any other error.
fn set_socket_option_supported(
    socket: &impl AsRawFd,
    level: libc::c_int,
    name: libc::c_int,
    value: libc::c_int,
) -> io::Result<bool> {
    match set_socket_option(socket, level, name, value) {
        Ok(()) => Ok(true),
        Err(err) if err.raw_os_error() == Some(libc::ENOPROTOOPT) => Ok(false),
        Err(err) => Err(err),
    }
}

// Defined in netinet6/in6.h on OpenBSD, this is not yet exported by the libc crate
// directly.  See https://github.com/rust-lang/libc/issues/3704 for when we might be able to
// rely on this from the libc crate.
#[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
const IPV6_DONTFRAG: libc::c_int = 62;
#[cfg(not(any(target_os = "openbsd", target_os = "netbsd")))]
const IPV6_DONTFRAG: libc::c_int = libc::IPV6_DONTFRAG;

#[cfg(target_os = "linux")]
mod gro {
    use super::*;

    pub(crate) fn gro_segments() -> usize {
        let socket = match std::net::UdpSocket::bind("[::]:0")
            .or_else(|_| std::net::UdpSocket::bind("127.0.0.1:0"))
        {
            Ok(socket) => socket,
            Err(_) => return 1,
        };

        // As defined in net/ipv4/udp_offload.c
        // #define UDP_GRO_CNT_MAX 64
        //
        // NOTE: this MUST be set to UDP_GRO_CNT_MAX to ensure that the receive buffer size
        // (get_max_udp_payload_size() * gro_segments()) is large enough to hold the largest GRO
        // list the kernel might potentially produce. See
        // https://github.com/quinn-rs/quinn/pull/1354.
        match set_socket_option(&socket, libc::SOL_UDP, libc::UDP_GRO, OPTION_ON) {
            Ok(()) => 64,
            Err(_) => 1,
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
/// Chosen somewhat arbitrarily; might benefit from additional tuning.
/// todo: add the macos condition.
pub(crate) const BATCH_SIZE: usize = 32;

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "openbsd")))]
fn recv(io: SockRef<'_>, bufs: &mut [IoSliceMut<'_>], meta: &mut [RecvMeta]) -> io::Result<usize> {
    todo!()
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
fn send(
    #[allow(unused_variables)] // only used on Linux
    state: &UdpSocketState,
    io: SockRef<'_>,
    transmit: &Transmit<'_>,
) -> io::Result<()> {
    #[allow(unused_mut)] // only mutable on FreeBSD
    let mut encode_src_ip = true;
    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
    let mut msg_hdr: libc::msghdr = unsafe { mem::zeroed() };
    let mut iovec: libc::iovec = unsafe { mem::zeroed() };
    let mut cmsgs = cmsg::Aligned([0u8; CMSG_LEN]);
    let dst_addr = socket2::SockAddr::from(transmit.destination);
    prepare_msg(
        transmit,
        &dst_addr,
        &mut msg_hdr,
        &mut iovec,
        &mut cmsgs,
        encode_src_ip,
        state.sendmsg_einval(),
    );
    loop {
        let n = unsafe { libc::sendmsg(io.as_raw_fd(), &msg_hdr, 0) };
        if n == -1 {
            todo!()
        }
        return Ok(());
    }
}

fn prepare_msg(
    transmit: &Transmit<'_>,
    dst_addr: &socket2::SockAddr,
    hdr: &mut libc::msghdr,
    iov: &mut libc::iovec,
    ctrl: &mut cmsg::Aligned<[u8; CMSG_LEN]>,
    #[allow(unused_variables)] // only used on FreeBSD & macOS
    encode_src_ip: bool,
    sendmsg_einval: bool,
) {
    iov.iov_base = transmit.contents.as_ptr() as *const _ as *mut _;
    iov.iov_len = transmit.contents.len();

    // SAFETY: Casting the pointer to a mutable one is legal,
    // as sendmsg is guaranteed to not alter the mutable pointer
    // as per the POSIX spec. See the section on the sys/socket.h
    // header for details. The type is only mutable in the first
    // place because it is reused by recvmsg as well.
    let name = dst_addr.as_ptr() as *mut libc::c_void;
    let namelen = dst_addr.len();
    hdr.msg_name = name as *mut _;
    hdr.msg_namelen = namelen;
    hdr.msg_iov = iov;
    hdr.msg_iovlen = 1;

    hdr.msg_control = ctrl.0.as_mut_ptr() as _;
    hdr.msg_controllen = CMSG_LEN as _;
    let mut encoder = unsafe { cmsg::Encoder::new(hdr) };
    let ecn = transmit.ecn.map_or(0, |x| x as libc::c_int);
    // True for IPv4 or IPv4-Mapped IPv6
    let is_ipv4 = transmit.destination.is_ipv4()
        || matches!(transmit.destination.ip(), IpAddr::V6(addr) if addr.to_ipv4_mapped().is_some());
    if is_ipv4 {
        if !sendmsg_einval {
            #[cfg(not(target_os = "netbsd"))]
            {
                encoder.push(libc::IPPROTO_IP, libc::IP_TOS, ecn as IpTosTy);
            }
        }
    } else {
        todo!() // encoder.push(libc::IPPROTO_IPV6, libc::IPV6_TCLASS, ecn);
    }
    if let Some(segment_size) = transmit.segment_size {
        todo!() // gso::set_segment_size(&mut encoder, segment_size as u16);
    }
    if let Some(ip) = &transmit.src_ip {
        todo!()
    }

    encoder.finish();
}
