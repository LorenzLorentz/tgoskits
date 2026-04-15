use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use core::task::Waker;

use ax_kspin::SpinNoIrq;
use ax_task::future::register_irq_waker;
use lazy_static::lazy_static;

use super::{
    Tty,
    terminal::ldisc::{ProcessMode, TtyConfig, TtyRead, TtyWrite},
};

pub type NTtyDriver = Tty<Console, Console>;

#[derive(Clone, Copy, Default)]
enum ConsoleEscapeState {
    #[default]
    None,
    Esc,
    Csi {
        len: usize,
        buf: [u8; 8],
    },
}

#[derive(Clone, Copy)]
pub struct Console;
impl TtyRead for Console {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut read = 0;
        let mut queue = CONSOLE_REPLY_BYTES.lock();
        while read < buf.len() {
            let Some(byte) = queue.pop_front() else {
                break;
            };
            buf[read] = byte;
            read += 1;
        }
        drop(queue);
        read += ax_hal::console::read_bytes(&mut buf[read..]);
        read
    }
}
impl TtyWrite for Console {
    fn write(&self, buf: &[u8]) {
        let mut out = alloc::vec::Vec::with_capacity(buf.len());
        let mut state = CONSOLE_ESCAPE_STATE.lock();

        for &byte in buf {
            match *state {
                ConsoleEscapeState::None => {
                    if byte == 0x1b {
                        *state = ConsoleEscapeState::Esc;
                    } else {
                        out.push(byte);
                    }
                }
                ConsoleEscapeState::Esc => {
                    if byte == b'[' {
                        *state = ConsoleEscapeState::Csi {
                            len: 0,
                            buf: [0; 8],
                        };
                    } else {
                        out.push(0x1b);
                        out.push(byte);
                        *state = ConsoleEscapeState::None;
                    }
                }
                ConsoleEscapeState::Csi { mut len, mut buf } => {
                    if (0x40..=0x7e).contains(&byte) {
                        if byte == b'n' && len == 1 && buf[0] == b'6' {
                            CONSOLE_REPLY_BYTES.lock().extend(b"\x1b[1;1R");
                        } else {
                            out.push(0x1b);
                            out.push(b'[');
                            out.extend_from_slice(&buf[..len]);
                            out.push(byte);
                        }
                        *state = ConsoleEscapeState::None;
                    } else {
                        if len < buf.len() {
                            buf[len] = byte;
                            len += 1;
                        }
                        *state = ConsoleEscapeState::Csi { len, buf };
                    }
                }
            }
        }

        if !out.is_empty() {
            ax_hal::console::write_bytes(&out);
        }
    }
}

lazy_static! {
    static ref CONSOLE_REPLY_BYTES: SpinNoIrq<VecDeque<u8>> = SpinNoIrq::new(VecDeque::new());
    static ref CONSOLE_ESCAPE_STATE: SpinNoIrq<ConsoleEscapeState> =
        SpinNoIrq::new(ConsoleEscapeState::default());
    /// The default TTY device.
    pub static ref N_TTY: Arc<NTtyDriver> = new_n_tty();
}

fn new_n_tty() -> Arc<NTtyDriver> {
    Tty::new(
        Arc::default(),
        TtyConfig {
            reader: Console,
            writer: Console,
            process_mode: ProcessMode::External(if let Some(irq) = ax_hal::console::irq_num() {
                Box::new(move |waker| register_irq_waker(irq, &waker)) as _
            } else {
                Box::new(|waker: Waker| waker.wake_by_ref()) as _
            }),
        },
    )
}
