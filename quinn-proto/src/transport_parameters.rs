use std::net::{SocketAddrV4, SocketAddrV6};

use crate::{endpoint::TransportConfig, shared::ConnectionId, ResetToken, VarInt};
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

impl TransportParameters {
    pub(crate) fn new(config: &TransportConfig) -> Self {
        Self { ..Self::default() }
    }
}
