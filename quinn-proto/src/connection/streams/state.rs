use rustc_hash::FxHashMap;
use tracing::trace;

use crate::{
    connection::{
        spaces::{Retransmits, ThinRetransmits},
        stats::FrameStats,
    },
    frame::{self, FrameStruct, StreamMetaVec},
    transport_parameters::TransportParameters,
    Dir, Side, StreamId, TransportError, VarInt,
};

use super::{
    recv::Recv,
    send::{Send, SendState},
    PendingStreamsQueue, ShouldTransmit, StreamEvent, StreamHalf,
};
use std::{
    collections::{hash_map, VecDeque},
    mem,
};

#[allow(unreachable_pub)] // fuzzing only
pub struct StreamsState {
    /// Whether the remote endpoint has opened any streams the application doesn't know about yet,
    /// per directionality
    opened: [bool; 2],
    /// Streams blocked on connection-level flow control or stream window space
    ///
    /// Streams are only added to this list when a write fails.
    pub(super) connection_blocked: Vec<StreamId>,

    // Set of streams that are currently open, or could be immediately opened by the peer
    pub(super) send: FxHashMap<StreamId, Option<Box<Send>>>,

    events: VecDeque<StreamEvent>,
    /// Connection-level flow control budget dictated by the peer
    pub(super) max_data: u64,
    /// Sum of current offsets of all send streams.
    pub(super) data_sent: u64,
    /// Total quantity of unacknowledged outgoing data
    pub(super) unacked_data: u64,
    /// Configured upper bound for `unacked_data`
    pub(super) send_window: u64,
    /// Pertinent state from the TransportParameters supplied by the peer
    initial_max_stream_data_uni: VarInt,
    initial_max_stream_data_bidi_local: VarInt,
    initial_max_stream_data_bidi_remote: VarInt,

    /// Maximum number of locally-initiated streams that may be opened over the lifetime of the
    /// connection so far, per direction
    pub(super) max: [u64; 2],
    /// Maximum number of remotely-initiated streams that may be opened over the lifetime of the
    /// connection so far, per direction
    pub(super) max_remote: [u64; 2],
    pub(super) side: Side,
    /// Streams with outgoing data queued, sorted by priority
    pub(super) pending: PendingStreamsQueue,
    /// Value of `max_remote` most recently transmitted to the peer in a `MAX_STREAMS` frame
    sent_max_remote: [u64; 2],
    /// Size of the desired stream flow control window. May be smaller than `allocated_remote_count`
    /// due to `set_max_concurrent` calls.
    max_concurrent_remote_count: [u64; 2],

    pub(super) next: [u64; 2],
    pub(super) recv: FxHashMap<StreamId, Option<Box<Recv>>>,
    /// Number of outbound streams
    ///
    /// This differs from `self.send.len()` in that it does not include streams that the peer is
    /// permitted to open but which have not yet been opened.
    pub(super) send_streams: usize,
}

impl StreamsState {
    #[allow(unreachable_pub)] // fuzzing only
    pub fn new(
        side: Side,
        max_remote_uni: VarInt,
        max_remote_bi: VarInt,
        send_window: u64,
        receive_window: VarInt,
        stream_receive_window: VarInt,
    ) -> Self {
        let mut this = Self {
            opened: [false, false],
            connection_blocked: Vec::new(),
            send: FxHashMap::default(),
            events: VecDeque::new(),
            max_data: 0,
            data_sent: 0,
            unacked_data: 0,
            send_window,
            initial_max_stream_data_uni: 0u32.into(),
            initial_max_stream_data_bidi_local: 0u32.into(),
            initial_max_stream_data_bidi_remote: 0u32.into(),
            max: [0, 0],
            max_remote: [max_remote_bi.into(), max_remote_uni.into()],
            side,
            pending: PendingStreamsQueue::new(),

            sent_max_remote: [max_remote_bi.into(), max_remote_uni.into()],
            // allocated_remote_count: [max_remote_bi.into(), max_remote_uni.into()],
            max_concurrent_remote_count: [max_remote_bi.into(), max_remote_uni.into()],
            next: [0, 0],

            recv: FxHashMap::default(),
            send_streams: 0,
        };

        for dir in Dir::iter() {
            for i in 0..this.max_remote[dir as usize] {
                this.insert(true, StreamId::new(!side, dir, i));
            }
        }
        this
    }

    /// 2. Yield stream events
    pub(crate) fn poll(&mut self) -> Option<StreamEvent> {
        if let Some(dir) = Dir::iter().find(|&i| mem::replace(&mut self.opened[i as usize], false))
        {
            return Some(StreamEvent::Opened { dir });
        }
        if self.write_limit() > 0 {
            while let Some(id) = self.connection_blocked.pop() {
                let stream = match self.send.get_mut(&id).and_then(|s| s.as_mut()) {
                    None => continue,
                    Some(s) => s,
                };

                debug_assert!(stream.connection_blocked);
                stream.connection_blocked = false;

                // If it's no longer sensible to write to a stream (even to detect an error) then don't
                // report it.
                if stream.is_writable() && stream.max_data > stream.offset() {
                    return Some(StreamEvent::Writable { id });
                }
            }
        }

        self.events.pop_front()
    }

    /// 3.Returns the maximum amount of data this is allowed to be written on the connection
    pub(crate) fn write_limit(&self) -> u64 {
        (self.max_data - self.data_sent).min(self.send_window - self.unacked_data)
    }
    /// 4.
    pub(in crate::connection) fn write_control_frames(
        &mut self,
        buf: &mut Vec<u8>,
        pending: &mut Retransmits,
        retransmits: &mut ThinRetransmits,
        stats: &mut FrameStats,
        max_size: usize,
    ) {
        // RESET_STREAM
        while buf.len() + frame::ResetStream::SIZE_BOUND < max_size {
            let (id, error_code) = match pending.reset_stream.pop() {
                Some(x) => x,
                None => break,
            };
            let stream = match self.send.get_mut(&id).and_then(|s| s.as_mut()) {
                Some(x) => x,
                None => continue,
            };
            trace!(stream = %id, "RESET_STREAM");
            retransmits
                .get_or_create()
                .reset_stream
                .push((id, error_code));
            frame::ResetStream {
                id,
                error_code,
                final_offset: VarInt::try_from(stream.offset()).expect("impossibly large offset"),
            }
            .encode(buf);
            stats.reset_stream += 1;
        }

        // STOP_SENDING
        while buf.len() + frame::StopSending::SIZE_BOUND < max_size {
            let frame = match pending.stop_sending.pop() {
                Some(x) => x,
                None => break,
            };
            // We may need to transmit STOP_SENDING even for streams whose state we have discarded,
            // because we are able to discard local state for stopped streams immediately upon
            // receiving FIN, even if the peer still has arbitrarily large amounts of data to
            // (re)transmit due to loss or unconventional sending strategy. We could fine-tune this
            // a little by dropping the frame if we specifically know the stream's been reset by the
            // peer, but we discard that information as soon as the application consumes it, so it
            // can't be relied upon regardless.
            trace!(stream = %frame.id, "STOP_SENDING");
            frame.encode(buf);
            retransmits.get_or_create().stop_sending.push(frame);
            stats.stop_sending += 1;
        }

        // MAX_DATA
        if pending.max_data && buf.len() + 9 < max_size {
            todo!()
        }

        // MAX_STREAM_DATA
        while buf.len() + 17 < max_size {
            let id = match pending.max_stream_data.iter().next() {
                Some(x) => *x,
                None => break,
            };
            todo!()
        }

        // MAX_STREAMS
        for dir in Dir::iter() {
            trace!("for dir in Dir::iter()");
            if !pending.max_stream_id[dir as usize] || buf.len() + 9 >= max_size {
                continue;
            }
            todo!()
        }
    }
    /// 5.
    pub(crate) fn write_stream_frames(
        &mut self,
        buf: &mut Vec<u8>,
        max_buf_size: usize,
    ) -> StreamMetaVec {
        let mut stream_frames = StreamMetaVec::new();
        while buf.len() + frame::Stream::SIZE_BOUND < max_buf_size {
            if max_buf_size
                .checked_sub(buf.len() + frame::Stream::SIZE_BOUND)
                .is_none()
            {
                break;
            }

            // Pop the stream of the highest priority that currently has pending data
            // If the stream still has some pending data left after writing, it will be reinserted, otherwise not
            let Some(stream) = self.pending.streams.pop() else {
                break;
            };
            todo!()
        }
        stream_frames
    }
    /// 6.
    pub(crate) fn set_params(&mut self, params: &TransportParameters) {
        self.initial_max_stream_data_uni = params.initial_max_stream_data_uni;
        self.initial_max_stream_data_bidi_local = params.initial_max_stream_data_bidi_local;
        self.initial_max_stream_data_bidi_remote = params.initial_max_stream_data_bidi_remote;
        self.max[Dir::Bi as usize] = params.initial_max_streams_bidi.into();
        self.max[Dir::Uni as usize] = params.initial_max_streams_uni.into();
        self.received_max_data(params.initial_max_data);
        for i in 0..self.max_remote[Dir::Bi as usize] {
            let id = StreamId::new(!self.side, Dir::Bi, i);
            if let Some(s) = self.send.get_mut(&id).and_then(|s| s.as_mut()) {
                s.max_data = params.initial_max_stream_data_bidi_local.into();
            }
        }
    }

    /// 7. Handle increase to connection-level flow control limit
    pub(crate) fn received_max_data(&mut self, n: VarInt) {
        self.max_data = self.max_data.max(n.into());
    }
    /// 8. Whether any stream data is queued, regardless of control frames
    pub(crate) fn can_send_stream_data(&self) -> bool {
        // Reset streams may linger in the pending stream list, but will never produce stream frames
        self.pending.streams.iter().any(|stream| {
            self.send
                .get(&stream.id)
                .and_then(|s| s.as_ref())
                .map_or(false, |s| !s.is_reset())
        })
    }
    /// 9.
    pub(crate) fn reset_acked(&mut self, id: StreamId) {
        match self.send.entry(id) {
            hash_map::Entry::Vacant(_) => {}
            hash_map::Entry::Occupied(e) => {
                if let Some(SendState::ResetSent) = e.get().as_ref().map(|s| s.state) {
                    e.remove_entry();
                    self.stream_freed(id, StreamHalf::Send);
                }
            }
        }
    }
    /// 10. Update counters for removal of a stream
    pub(super) fn stream_freed(&mut self, id: StreamId, half: StreamHalf) {
        todo!()
    }
    /// 11.
    pub(crate) fn received_ack_of(&mut self, frame: frame::StreamMeta) {
        todo!()
    }
    /// 12. Process incoming stream frame
    ///
    /// If successful, returns whether a `MAX_DATA` frame needs to be transmitted
    pub(crate) fn received(
        &mut self,
        frame: frame::Stream,
        payload_len: usize,
    ) -> Result<ShouldTransmit, TransportError> {
        todo!()
    }
    /// 13. Queues MAX_STREAM_ID frames in `pending` if needed
    ///
    /// Returns whether any frames were queued.
    pub(crate) fn queue_max_stream_id(&mut self, pending: &mut Retransmits) -> bool {
        let mut queued = false;
        for dir in Dir::iter() {
            let diff = self.max_remote[dir as usize] - self.sent_max_remote[dir as usize];
            // To reduce traffic, only announce updates if at least 1/8 of the flow control window
            // has been consumed.
            if diff > self.max_concurrent_remote_count[dir as usize] / 8 {
                pending.max_stream_id[dir as usize] = true;
                queued = true;
            }
        }
        queued
    }
    /// 14.
    pub(crate) fn retransmit_all_for_0rtt(&mut self) {
        for dir in Dir::iter() {
            for index in 0..self.next[dir as usize] {
                let id = StreamId::new(Side::Client, dir, index);
                let stream = match self.send.get_mut(&id).and_then(|s| s.as_mut()) {
                    Some(stream) => stream,
                    None => continue,
                };
                if stream.pending.is_fully_acked() && !stream.fin_pending {
                    // Stream data can't be acked in 0-RTT, so we must not have sent anything on
                    // this stream
                    continue;
                }
                if !stream.is_pending() {
                    self.pending.push_pending(id, stream.priority);
                }
                stream.pending.retransmit_all_for_0rtt();
            }
        }
    }
    /// 15
    pub(super) fn insert(&mut self, remote: bool, id: StreamId) {
        let bi = id.dir() == Dir::Bi;
        // bidirectional OR (unidirectional AND NOT remote)
        if bi || !remote {
            assert!(self.send.insert(id, None).is_none());
        }
        // bidirectional OR (unidirectional AND remote)
        if bi || remote {
            assert!(self.recv.insert(id, None).is_none());
        }
    }

    /// 16. Whether MAX_STREAM_DATA frames could be sent for stream `id`
    pub(crate) fn can_send_flow_control(&self, id: StreamId) -> bool {
        self.recv
            .get(&id)
            .and_then(|s| s.as_ref())
            .map_or(false, |s| s.can_send_flow_control())
    }
}
