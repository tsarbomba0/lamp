mod task;
use crate::task::Notification;
use crate::task::waker::make_waker;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll};
use task::task::Task;

struct Dummy {
    ready: bool,
}

impl Future for Dummy {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.ready {
            println!("Ready!");
            Poll::Ready(())
        } else {
            println!("Pending!");
            self.ready = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn main() {
    println!("Okay start!");
    let (sender, receiver) = mpsc::channel();

    let test = Dummy { ready: false };

    let (task, notif) = Task::new(test, sender);
    let waker = make_waker(Arc::new(notif));

    let mut poll: Poll<()> = Poll::Pending;
    task.attach_waker(&waker);

    task.poll();

    task.raw.read_output(&mut poll as *mut _ as *mut ());

    dbg!(poll.is_ready());

    if let Ok(nt) = receiver.recv() {
        println!("Got notification!");
        nt.test();
    };

    task.raw.read_output(&mut poll as *mut _ as *mut ());
    dbg!(poll.is_ready());
}
