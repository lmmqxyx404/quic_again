use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use bytes::{Buf, BufMut};
use thiserror::Error;

use crate::{
    cid_queue::CidQueue,
    coding::{BufExt, BufMutExt, UnexpectedEnd},
    config::{EndpointConfig, ServerConfig, TransportConfig},
    shared::ConnectionId,
    ConnectionIdGenerator, ResetToken, Side, TransportError, VarInt, LOC_CID_COUNT, MAX_CID_SIZE,
    MAX_STREAM_COUNT, RESET_TOKEN_SIZE, TIMER_GRANULARITY,
};
// Apply a given macro to a list of all the transport parameters having integer types, along with
// their codes and default values. Using this helps us avoid error-prone duplication of the
// contained information across decoding, encoding, and the `Default` impl. Whenever we want to do
// something with transport parameters, we'll handle the bulk of cases by writing a macro that
// takes a list of arguments in this form, then passing it to this macro.
macro_rules! apply_params {
    ($macro:ident) => {
        $macro! {
            // #[doc] name (id) = default,
            /// Milliseconds, disabled if zero
            max_idle_timeout(0x0001) = 0,
            /// Limits the size of UDP payloads that the endpoint is willing to receive
            max_udp_payload_size(0x0003) = 65527,

            /// Initial value for the maximum amount of data that can be sent on the connection
            initial_max_data(0x0004) = 0,
            /// Initial flow control limit for locally-initiated bidirectional streams
            initial_max_stream_data_bidi_local(0x0005) = 0,
            /// Initial flow control limit for peer-initiated bidirectional streams
            initial_max_stream_data_bidi_remote(0x0006) = 0,
            /// Initial flow control limit for unidirectional streams
            initial_max_stream_data_uni(0x0007) = 0,

            /// Initial maximum number of bidirectional streams the peer may initiate
            initial_max_streams_bidi(0x0008) = 0,
            /// Initial maximum number of unidirectional streams the peer may initiate
            initial_max_streams_uni(0x0009) = 0,

            /// Exponent used to decode the ACK Delay field in the ACK frame
            ack_delay_exponent(0x000a) = 3,
            /// Maximum amount of time in milliseconds by which the endpoint will delay sending
            /// acknowledgments
            max_ack_delay(0x000b) = 25,
            /// Maximum number of connection IDs from the peer that an endpoint is willing to store
            active_connection_id_limit(0x000e) = 2,
        }
    };
}

macro_rules! make_struct {
    {$($(#[$doc:meta])* $name:ident ($code:expr) = $default:expr,)*} => {
        /// Transport parameters used to negotiate connection-level preferences between peers
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct TransportParameters {
            $($(#[$doc])* pub(crate) $name : VarInt,)*

            /// Does the endpoint support active connection migration
            pub(crate) disable_active_migration: bool,
            /// Maximum size for datagram frames
            pub(crate) max_datagram_frame_size: Option<VarInt>,
            /// The value that the endpoint included in the Source Connection ID field of the first
            /// Initial packet it sends for the connection
            pub(crate) initial_src_cid: Option<ConnectionId>,
            /// The endpoint is willing to receive QUIC packets containing any value for the fixed
            /// bit
            pub(crate) grease_quic_bit: bool,

            /// Minimum amount of time in microseconds by which the endpoint is able to delay
            /// sending acknowledgments
            ///
            /// If a value is provided, it implies that the endpoint supports QUIC Acknowledgement
            /// Frequency
            pub(crate) min_ack_delay: Option<VarInt>,

            // Server-only
            /// The value of the Destination Connection ID field from the first Initial packet sent
            /// by the client
            pub(crate) original_dst_cid: Option<ConnectionId>,
            /// The value that the server included in the Source Connection ID field of a Retry
            /// packet
            pub(crate) retry_src_cid: Option<ConnectionId>,
            /// Token used by the client to verify a stateless reset from the server
            pub(crate) stateless_reset_token: Option<ResetToken>,
            /// The server's preferred address for communication after handshake completion
            pub(crate) preferred_address: Option<PreferredAddress>,
        }

        // We deliberately don't implement the `Default` trait, since that would be public, and
        // downstream crates should never construct `TransportParameters` except by decoding those
        // supplied by a peer.
        impl TransportParameters {
            /// Standard defaults, used if the peer does not supply a given parameter.
            pub(crate) fn default() -> Self {
                Self {
                    $($name: VarInt::from_u32($default),)*

                    disable_active_migration: false,
                    max_datagram_frame_size: None,
                    initial_src_cid: None,
                    grease_quic_bit: false,
                    min_ack_delay: None,

                    original_dst_cid: None,
                    retry_src_cid: None,
                    stateless_reset_token: None,
                    preferred_address: None,
                }
            }
        }
    }
}

apply_params!(make_struct);

/// A server's preferred address
///
/// This is communicated as a transport parameter during TLS session establishment.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct PreferredAddress {
    pub(crate) address_v4: Option<SocketAddrV4>,
    pub(crate) address_v6: Option<SocketAddrV6>,
    pub(crate) connection_id: ConnectionId,
    pub(crate) stateless_reset_token: ResetToken,
}

impl PreferredAddress {
    /// 1
    fn wire_size(&self) -> u16 {
        4 + 2 + 16 + 2 + 1 + self.connection_id.len() as u16 + 16
    }
    /// 2
    fn write<W: BufMut>(&self, w: &mut W) {
        w.write(self.address_v4.map_or(Ipv4Addr::UNSPECIFIED, |x| *x.ip()));
        w.write::<u16>(self.address_v4.map_or(0, |x| x.port()));
        w.write(self.address_v6.map_or(Ipv6Addr::UNSPECIFIED, |x| *x.ip()));
        w.write::<u16>(self.address_v6.map_or(0, |x| x.port()));
        w.write::<u8>(self.connection_id.len() as u8);
        w.put_slice(&self.connection_id);
        w.put_slice(&self.stateless_reset_token);
    }
    /// 3
    fn read<R: Buf>(r: &mut R) -> Result<Self, Error> {
        todo!()
    }
}

impl TransportParameters {
    /// 1
    pub(crate) fn new(
        config: &TransportConfig,
        endpoint_config: &EndpointConfig,
        cid_gen: &dyn ConnectionIdGenerator,
        initial_src_cid: ConnectionId,
        server_config: Option<&ServerConfig>,
    ) -> Self {
        Self {
            initial_src_cid: Some(initial_src_cid),
            initial_max_streams_uni: config.max_concurrent_uni_streams,
            initial_max_data: config.receive_window,
            initial_max_stream_data_uni: config.stream_receive_window,

            // following two are used for `key_update_simple`
            initial_max_streams_bidi: config.max_concurrent_bidi_streams,
            initial_max_stream_data_bidi_remote: config.stream_receive_window,
            // used for `idle_timeout`
            max_idle_timeout: config.max_idle_timeout.unwrap_or(VarInt(0)),
            // used for `migration`
            min_ack_delay: Some(
                VarInt::from_u64(u64::try_from(TIMER_GRANULARITY.as_micros()).unwrap()).unwrap(),
            ),
            // used for 
            active_connection_id_limit: if cid_gen.cid_len() == 0 {
                2 // i.e. default, i.e. unsent
            } else {
                CidQueue::LEN as u32
            }
            .into(),
            ..Self::default()
        }
    }

    /// Maximum number of CIDs to issue to this peer
    ///
    /// Consider both a) the active_connection_id_limit from the other end; and
    /// b) LOC_CID_COUNT used locally
    pub(crate) fn issue_cids_limit(&self) -> u64 {
        self.active_connection_id_limit.0.min(LOC_CID_COUNT)
    }

    /// Check that these parameters are legal when resuming from
    /// certain cached parameters
    pub(crate) fn validate_resumption_from(&self, cached: &Self) -> Result<(), TransportError> {
        if cached.active_connection_id_limit > self.active_connection_id_limit
            || cached.initial_max_data > self.initial_max_data
            || cached.initial_max_stream_data_bidi_local > self.initial_max_stream_data_bidi_local
            || cached.initial_max_stream_data_bidi_remote > self.initial_max_stream_data_bidi_remote
            || cached.initial_max_stream_data_uni > self.initial_max_stream_data_uni
            || cached.initial_max_streams_bidi > self.initial_max_streams_bidi
            || cached.initial_max_streams_uni > self.initial_max_streams_uni
            || cached.max_datagram_frame_size > self.max_datagram_frame_size
            || cached.grease_quic_bit && !self.grease_quic_bit
        {
            return Err(TransportError::PROTOCOL_VIOLATION(
                "0-RTT accepted with incompatible transport parameters",
            ));
        }
        Ok(())
    }
}

impl TransportParameters {
    /// 2. Encode `TransportParameters` into buffer
    pub fn write<W: BufMut>(&self, w: &mut W) {
        macro_rules! write_params {
            {$($(#[$doc:meta])* $name:ident ($code:expr) = $default:expr,)*} => {
                $(
                    if self.$name.0 != $default {
                        w.write_var($code);
                        w.write(VarInt::try_from(self.$name.size()).unwrap());
                        w.write(self.$name);
                    }
                )*
            }
        }
        apply_params!(write_params);

        // Add a reserved parameter to keep people on their toes
        w.write_var(31 * 5 + 27);
        w.write_var(0);

        if let Some(ref x) = self.stateless_reset_token {
            w.write_var(0x02);
            w.write_var(16);
            w.put_slice(x);
        }

        if self.disable_active_migration {
            w.write_var(0x0c);
            w.write_var(0);
        }

        if let Some(x) = self.max_datagram_frame_size {
            w.write_var(0x20);
            w.write_var(x.size() as u64);
            w.write(x);
        }

        if let Some(ref x) = self.preferred_address {
            w.write_var(0x000d);
            w.write_var(x.wire_size() as u64);
            x.write(w);
        }

        for &(tag, cid) in &[
            (0x00, &self.original_dst_cid),
            (0x0f, &self.initial_src_cid),
            (0x10, &self.retry_src_cid),
        ] {
            if let Some(ref cid) = *cid {
                w.write_var(tag);
                w.write_var(cid.len() as u64);
                w.put_slice(cid);
            }
        }

        if self.grease_quic_bit {
            w.write_var(0x2ab2);
            w.write_var(0);
        }

        if let Some(x) = self.min_ack_delay {
            w.write_var(0xff04de1a);
            w.write_var(x.size() as u64);
            w.write(x);
        }
    }

    /// Decode `TransportParameters` from buffer
    pub fn read<R: Buf>(side: Side, r: &mut R) -> Result<Self, Error> {
        // Initialize to protocol-specified defaults
        let mut params = Self::default();

        // State to check for duplicate transport parameters.
        macro_rules! param_state {
                 {$($(#[$doc:meta])* $name:ident ($code:expr) = $default:expr,)*} => {{
                     struct ParamState {
                         $($name: bool,)*
                     }

                     ParamState {
                         $($name: false,)*
                     }
                 }}
             }
        let mut got = apply_params!(param_state);
        while r.has_remaining() {
            // pay attention to the following code
            let id = r.get_var()?;
            let len = r.get_var()?;
            if (r.remaining() as u64) < len {
                return Err(Error::Malformed);
            }

            let len = len as usize;

            #[cfg(test)]
            {
                tracing::info!("TransportParameters id is {}", id);
            }
            match id {
                0x00 => decode_cid(len, &mut params.original_dst_cid, r)?,
                0x02 => {
                    if len != 16 || params.stateless_reset_token.is_some() {
                        return Err(Error::Malformed);
                    }
                    let mut tok = [0; RESET_TOKEN_SIZE];
                    r.copy_to_slice(&mut tok);
                    params.stateless_reset_token = Some(tok.into());
                }
                0x0c => {
                    if len != 0 || params.disable_active_migration {
                        return Err(Error::Malformed);
                    }
                    params.disable_active_migration = true;
                }
                0x0d => {
                    if params.preferred_address.is_some() {
                        return Err(Error::Malformed);
                    }
                    params.preferred_address = Some(PreferredAddress::read(&mut r.take(len))?);
                }
                0x0f => decode_cid(len, &mut params.initial_src_cid, r)?,
                0x10 => decode_cid(len, &mut params.retry_src_cid, r)?,
                0x20 => {
                    if len > 8 || params.max_datagram_frame_size.is_some() {
                        return Err(Error::Malformed);
                    }
                    params.max_datagram_frame_size = Some(r.get().unwrap());
                }
                0x2ab2 => match len {
                    0 => params.grease_quic_bit = true,
                    _ => return Err(Error::Malformed),
                },
                0xff04de1a => params.min_ack_delay = Some(r.get().unwrap()),
                _ => {
                    #[cfg(test)]
                    {
                        tracing::info!("TransportParameters default id is {}", id);
                    }
                    macro_rules! parse {
                        {$($(#[$doc:meta])* $name:ident ($code:expr) = $default:expr,)*} => {
                            match id {
                                $($code => {
                                    let value = r.get::<VarInt>()?;
                                    if len != value.size() || got.$name { return Err(Error::Malformed); }
                                    params.$name = value.into();
                                    got.$name = true;
                                })*
                                _ => r.advance(len as usize),
                            }
                        }
                    }
                    apply_params!(parse);
                }
            }
        }
        // Semantic validation

        // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-4.26.1
        if params.ack_delay_exponent.0 > 20
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-4.28.1
            || params.max_ack_delay.0 >= 1 << 14
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-6.2.1
            || params.active_connection_id_limit.0 < 2
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-4.10.1
            || params.max_udp_payload_size.0 < 1200
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-4.6-2
            || params.initial_max_streams_bidi.0 > MAX_STREAM_COUNT
            || params.initial_max_streams_uni.0 > MAX_STREAM_COUNT
            // https://www.ietf.org/archive/id/draft-ietf-quic-ack-frequency-08.html#section-3-4
            || params.min_ack_delay.map_or(false, |min_ack_delay| {
                // min_ack_delay uses microseconds, whereas max_ack_delay uses milliseconds
                min_ack_delay.0 > params.max_ack_delay.0 * 1_000
            })
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-8
            || (side.is_server()
                && (params.original_dst_cid.is_some()
                    || params.preferred_address.is_some()
                    || params.retry_src_cid.is_some()
                    || params.stateless_reset_token.is_some()))
            // https://www.rfc-editor.org/rfc/rfc9000.html#section-18.2-4.38.1
            || params
                .preferred_address
                .map_or(false, |x| x.connection_id.is_empty())
        {
            return Err(Error::IllegalValue);
        }

        Ok(params)
    }
}

/// Errors encountered while decoding `TransportParameters`
#[derive(Debug, Copy, Clone, Eq, PartialEq, Error)]
pub enum Error {
    /// Parameters that are semantically invalid
    #[error("parameter had illegal value")]
    IllegalValue,
    /// Catch-all error for problems while decoding transport parameters
    #[error("parameters were malformed")]
    Malformed,
}

impl From<Error> for TransportError {
    fn from(e: Error) -> Self {
        match e {
            Error::IllegalValue => Self::TRANSPORT_PARAMETER_ERROR("illegal value"),
            Error::Malformed => Self::TRANSPORT_PARAMETER_ERROR("malformed"),
        }
    }
}

impl From<UnexpectedEnd> for Error {
    fn from(_: UnexpectedEnd) -> Self {
        Self::Malformed
    }
}

fn decode_cid(len: usize, value: &mut Option<ConnectionId>, r: &mut impl Buf) -> Result<(), Error> {
    if len > MAX_CID_SIZE || value.is_some() || r.remaining() < len {
        return Err(Error::Malformed);
    }

    *value = Some(ConnectionId::from_buf(r, len));
    Ok(())
}
