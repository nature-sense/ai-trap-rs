use std::thread;
use std::thread::JoinHandle;
use tokio::runtime::Runtime;

pub trait Actor where Self: 'static {
    async fn start(self) -> JoinHandle<()> where Self: Sized, Self: Send {
        let handle = thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                self.on_started().await;
            });
        });
        handle
    }
    
    
    fn _stop() {

    }
    
    //async fn on_started(state: Arc<Mutex<Self>>) where Self: Sized;
    async fn on_started(self);

}
