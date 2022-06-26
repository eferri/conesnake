use crate::board::Board;
use crate::util::{Error, Move};

use std::{fs::File, sync::Arc, thread};

use crossbeam_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
pub struct BoardMoveState {
    pub request: Board,
    pub moves: Vec<Move>,
}

pub struct GpuMove {
    state: Arc<GpuMoveState>,
    _read_handle: thread::JoinHandle<()>,
    _write_handle: thread::JoinHandle<()>,
}

struct ChanPair<T> {
    send: Sender<T>,
    recv: Receiver<T>,
}

struct GpuMoveState {
    req: ChanPair<BoardMoveState>,
    resp: Vec<ChanPair<Board>>,
    req_file: Mutex<File>,
    resp_file: Mutex<File>,
}

impl GpuMove {
    pub fn new(cap: usize, req_file: &str, resp_file: &str) -> Result<Self, Error> {
        let mut resp = Vec::with_capacity(cap);
        for _ in 0..cap {
            let (resp_s, resp_r) = bounded(100);
            resp.push(ChanPair {
                send: resp_s,
                recv: resp_r,
            });
        }

        let (req_s, req_r) = bounded(100);

        let req_file = File::open(req_file)?;
        let resp_file = File::open(resp_file)?;

        let req_file = Mutex::new(req_file);
        let resp_file = Mutex::new(resp_file);

        let state = Arc::new(GpuMoveState {
            req: ChanPair {
                send: req_s,
                recv: req_r,
            },
            resp,
            req_file,
            resp_file,
        });

        let thread_state = state.clone();
        let read_handle = thread::spawn(move || {
            thread_state.read_resp();
        });

        let thread_state = state.clone();
        let write_handle = thread::spawn(move || {
            thread_state.write_req();
        });

        Ok(GpuMove {
            state,
            _read_handle: read_handle,
            _write_handle: write_handle,
        })
    }

    pub fn send_request(&self, board: BoardMoveState) {
        self.state.req.send.send(board).unwrap()
    }

    pub fn get_idx(&self, idx: usize) -> Board {
        self.state.resp[idx].recv.recv().unwrap()
    }
}

impl GpuMoveState {
    fn write_req(&self) {}

    fn read_resp(&self) {}
}
