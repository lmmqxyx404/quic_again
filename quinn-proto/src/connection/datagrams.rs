use std::collections::VecDeque;

use crate::frame::Datagram;

#[derive(Default)]
pub(super) struct DatagramState {
    /// 1
    pub(super) outgoing: VecDeque<Datagram>,
    /// 2.
    pub(super) send_blocked: bool,
}

impl DatagramState {
    /// 1.Attempt to write a datagram frame into `buf`, consuming it from `self.outgoing`
    ///
    /// Returns whether a frame was written. At most `max_size` bytes will be written, including
    /// framing.
    pub(super) fn write(&mut self, buf: &mut Vec<u8>, max_size: usize) -> bool {
        let datagram = match self.outgoing.pop_front() {
            Some(x) => x,
            None => return false,
        };
        todo!()
    }
}
