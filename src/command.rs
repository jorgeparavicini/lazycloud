use std::pin::Pin;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::AppMsg;

pub type BoxFut = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone)]
pub struct CommandEnv {
    pub msg_tx: UnboundedSender<AppMsg>
}

pub trait Command: Send {
    fn spawn(self: Box<Self>, env: CommandEnv);
}

pub trait AsyncCommand: Send + 'static {
    fn run(self, env: CommandEnv) -> BoxFut;
}

impl<T: AsyncCommand> Command for T {
    fn spawn(self: Box<Self>, env: CommandEnv) {
        tokio::spawn(self.run(env));
    }
}
