use std::time::Instant;

use crate::connection::spaces::PacketSpace;
use crate::crypto::{HeaderKey, KeyPair, PacketKey};
use crate::packet::{Packet, PartialDecode, SpaceId};
use crate::token::ResetToken;
use crate::{TransportError, RESET_TOKEN_SIZE};

/// Removes header protection of a packet, or returns `None` if the packet was dropped
pub(super) fn unprotect_header(
    partial_decode: PartialDecode,
    spaces: &[PacketSpace; 3],
    zero_rtt_crypto: Option<&ZeroRttCrypto>,
    stateless_reset_token: Option<ResetToken>,
) -> Option<UnprotectHeaderResult> {
    let header_crypto = if partial_decode.is_0rtt() {
        if let Some(crypto) = zero_rtt_crypto {
            Some(&*crypto.header)
        } else {
            return None;
        }
    } else if let Some(space) = partial_decode.space() {
        if let Some(ref crypto) = spaces[space].crypto {
            Some(&*crypto.header.remote)
        } else {
            return None;
        }
    } else {
        // Unprotected packet
        None
    };

    let packet = partial_decode.data();
    let stateless_reset = packet.len() >= RESET_TOKEN_SIZE + 5
        && stateless_reset_token.as_deref() == Some(&packet[packet.len() - RESET_TOKEN_SIZE..]);

    match partial_decode.finish(header_crypto) {
        Ok(packet) => Some(UnprotectHeaderResult {
            packet: Some(packet),
            stateless_reset,
        }),
        Err(_) if stateless_reset => Some(UnprotectHeaderResult {
            packet: None,
            stateless_reset: true,
        }),
        Err(e) => {
            // trace!("unable to complete packet decoding: {}", e);
            None
        }
    }
}

/// 1.
pub(super) struct UnprotectHeaderResult {
    /// The packet with the now unprotected header (`None` in the case of stateless reset packets
    /// that fail to be decoded)
    pub(super) packet: Option<Packet>,
    /// Whether the packet was a stateless reset packet
    pub(super) stateless_reset: bool,
}

/// Decrypts a packet's body in-place
pub(super) fn decrypt_packet_body(
    packet: &mut Packet,
    spaces: &[PacketSpace; 3],
    zero_rtt_crypto: Option<&ZeroRttCrypto>,
    conn_key_phase: bool,
    prev_crypto: Option<&PrevCrypto>,
    next_crypto: Option<&KeyPair<Box<dyn PacketKey>>>,
) -> Result<Option<DecryptPacketResult>, Option<TransportError>> {
    if !packet.header.is_protected() {
        // Unprotected packets also don't have packet numbers
        return Ok(None);
    }

    let space = packet.header.space();
    let rx_packet = spaces[space].rx_packet;
    let number = packet.header.number().ok_or(None)?.expand(rx_packet + 1);
    let packet_key_phase = packet.header.key_phase();

    todo!()
}
/// 4.
pub(super) struct DecryptPacketResult {
    /// 1. Whether a locally initiated key update has been acknowledged by the peer
    pub(super) outgoing_key_update_acked: bool,
    /// 2. The packet number
    pub(super) number: u64,
}
/// 3.
pub(super) struct PrevCrypto {
    /// 1. The incoming packet that ends the interval for which these keys are applicable, and the time
    /// of its receipt.
    ///
    /// Incoming packets should be decrypted using these keys iff this is `None` or their packet
    /// number is lower. `None` indicates that we have not yet received a packet using newer keys,
    /// which implies that the update was locally initiated.
    pub(super) end_packet: Option<(u64, Instant)>,
    /// 2. Whether the following key phase is from a remotely initiated update that we haven't acked
    pub(super) update_unacked: bool,
}

/// 2.
pub(super) struct ZeroRttCrypto {
    pub(super) header: Box<dyn HeaderKey>,
    pub(super) packet: Box<dyn PacketKey>,
}
