use std::{any::TypeId, collections::HashMap, sync::OnceLock};

use tokio::sync::mpsc;

static BROKER: OnceLock<mpsc::Sender<RawMessage>> = OnceLock::new();

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("There is no receiver of this currently held by the Broker")]
    NoReceiver,
}

struct Wrapper<P> {
    receiver: mpsc::Receiver<RawMessage>,
    inner: P,
}

impl<P> Wrapper<P>
where
    P: PillarExt,
{
    fn recv(&mut self, message: RawMessage) {
        let ptr = message.ptr as *mut <P as Pillar>::Message;
        let message = unsafe { Box::from_raw(ptr) };
        self.inner.recv(message).unwrap();
    }

    async fn spawn(mut self) {
        loop {
            let msg = self.receiver.recv().await.unwrap();
            self.recv(msg);
        }
    }
}

struct RawMessage {
    id: TypeId,
    ptr: *mut usize,
}

unsafe impl Send for RawMessage {}

pub trait Message {
    type Response;
}

pub trait Pillar: 'static + Send {
    type Message: 'static + Send + Message;

    type Error;

    fn recv(
        &mut self,
        message: Box<Self::Message>,
    ) -> Result<<Self::Message as Message>::Response, Error>;

    fn spawn(self) -> Result<(), Self::Error>;
}

// Should prevent send from ever being reimplemented
impl<P: Pillar> PillarExt for P {}

pub trait PillarExt: Pillar {
    fn send<M>(&self, message: M) -> Result<<Self::Message as Message>::Response, Error>
    where
        M: 'static + Send + Message,
    {
        let leaked_message = Box::leak(Box::new(message));
        let invalid_message_ptr = leaked_message as *mut M as *mut usize;
        let message = RawMessage {
            id: TypeId::of::<M>(),
            ptr: invalid_message_ptr,
        };

        let global_sender = BROKER.get().unwrap();
        global_sender.blocking_send(message).unwrap();

        todo!()
    }
}

pub struct Broker {
    receiver: mpsc::Receiver<RawMessage>,
    pillars: HashMap<TypeId, mpsc::Sender<RawMessage>>,
}

impl Broker {
    async fn forward(&self, message: RawMessage) -> Result<(), Error> {
        let Some(sender) = self.pillars.get(&message.id) else {
            return Err(Error::NoReceiver);
        };

        if let Err(_) = sender.send(message).await {
            return Err(Error::NoReceiver);
        }

        Ok(())
    }

    pub fn add_pillar<P: Pillar>(&mut self, pillar: P) {
        let (tx, rx) = mpsc::channel(100);
        let id = TypeId::of::<<P as Pillar>::Message>();
        let wrapper = Wrapper {
            receiver: rx,
            inner: pillar,
        };

        self.pillars.insert(id, tx);
        tokio::spawn(async move { wrapper.spawn().await });
    }
}
