use super::IoSource;
use mio::event::Source;
use mio::{Events, Interest, Poll, Registry, Token};
use slab::Slab;
use std::io::Result as IoResult;
use std::sync::{Arc, Mutex};
use std::task::Context;

/// represents the interest of the underlying io.
pub enum Direction {
    Read,
    Write,
}

/// Represents the global I/O Reactor.
///
/// Only one exists at anytime.
pub struct Reactor {
    /// Re-usable event pool.
    events: Arc<Mutex<Events>>,

    /// Handle
    handle: Arc<Handle>,

    /// I/O sources
    sources: Arc<Mutex<Slab<IoSource>>>,
}

/// Handle to the I/O Reactor.
pub struct Handle {
    /// Registry belonging to `mio::Poll`
    registry: Registry,

    /// Poll from which we obtain events.
    poll: Mutex<Poll>,
}

impl Handle {
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    fn arc_new(registry: Registry, poll: Poll) -> Arc<Handle> {
        Arc::new(Handle {
            registry,
            poll: Mutex::new(poll),
        })
    }
}

impl Reactor {
    /// Create a new Reactor
    pub fn new() -> (Reactor, Arc<Handle>) {
        let poll = Poll::new().expect("failed to create poll");
        let events = Arc::new(Mutex::new(Events::with_capacity(1024)));
        let registry = poll.registry().try_clone().expect("registry clone fail");
        let sources = Arc::new(Mutex::new(Slab::with_capacity(1024)));

        let handle = Handle::arc_new(registry, poll);
        let r = Reactor {
            sources,
            events,
            handle,
        };
        let arc_handle = Arc::clone(&r.handle);
        (r, arc_handle)
    }

    /// Get reference to the global Reactor instance.
    pub fn start(&self) {
        // Polling thread
        let arc_events = Arc::clone(&self.events);
        let arc_sources = Arc::clone(&self.sources);
        let handle = Arc::clone(&self.handle);
        std::thread::spawn(move || {
            let mut poll = handle.poll.lock().expect("failed loop poll lock");
            let mut events = arc_events.lock().expect("event lock fail");

            loop {
                match poll.poll(&mut events, None) {
                    Ok(_) => {}
                    Err(e) => panic!("Error: {:?}", e),
                }

                for event in events.iter() {
                    println!("{:?}", event);
                    let srcs = arc_sources.lock().expect("sources lock in loop failed!");

                    let src = match srcs.get(event.token().0) {
                        None => panic!(
                            "Received event for token {}, but no such source is present.",
                            event.token().0
                        ),
                        Some(source) => source,
                    };

                    if src.has_wakers() {
                        src.wake_with_event(event)
                    }
                }
            }
        });
    }

    /// Obtains handle from a reactor.
    pub fn get_handle(&self) -> Arc<Handle> {
        self.handle.clone()
    }

    /// Registers a IO source in the reactor.
    pub fn register(&self, src: &mut impl Source, interest: Interest) -> IoResult<usize> {
        let mut sources = self.sources.lock().expect("failed source lock");
        let token = sources.vacant_key();

        self.handle.registry.register(src, Token(token), interest)?;

        let _ = sources.insert(IoSource::new(token));
        Ok(token)
    }

    /// Reregisters a IO source in the reactor.
    pub fn reregister(&self, src: &mut impl Source, token: usize, intr: Interest) -> IoResult<()> {
        //let sources = Reactor::get().sources.lock().expect("failed sources lock!");
        self.handle.registry.reregister(src, Token(token), intr)
    }

    pub fn attach_waker(&self, cx: &mut Context<'_>, token: Token, dir: Direction) {
        let mut sources = self.sources.lock().expect("failed sources lock!");
        let src = match sources.get_mut(token.0) {
            Some(source) => source,
            None => panic!("Trying to attach waker to an unregistered source!"),
        };

        match dir {
            Direction::Read => {
                let cur_waker = src.get_read_waker();
                match cur_waker {
                    None => src.change_read_waker(cx.waker()),
                    Some(waker) => {
                        if !waker.will_wake(cx.waker()) {
                            src.change_read_waker(cx.waker());
                        }
                    }
                }
            }
            Direction::Write => src.change_write_waker(cx.waker()),
        }
    }
}
