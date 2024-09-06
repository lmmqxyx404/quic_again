use super::{CMsgHdr, MsgHdr};

#[derive(Copy, Clone)]
#[repr(align(8))] // Conservative bound for align_of<libc::cmsghdr>
pub(crate) struct Aligned<T>(pub(crate) T);

/// Helpers for [`libc::msghdr`]
impl MsgHdr for libc::msghdr {
    type ControlMessage = libc::cmsghdr;

    fn cmsg_first_hdr(&self) -> *mut Self::ControlMessage {
        unsafe { libc::CMSG_FIRSTHDR(self) }
    }
}

/// Helpers for [`libc::cmsghdr`]
impl CMsgHdr for libc::cmsghdr {}
