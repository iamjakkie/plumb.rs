
pub struct SubscriptionMeta {
    pub ws_channels: Vec<String>, // defines channels we want to subscribe to
    pub ws_url: String // base URL
    /*
        some meta fields, e.g. subscription msg
     */
}

pub struct SubscriberManager {
    url: String,
    channel: String,
    subscribers: Vec<Arc<Mutex<Subscriber>>>,
    subscriptions: HashSet<String>,
    zmq_tx: mpsc::Sender<Vec<u8>>
}

pub struct Subscriber {
    url: String,
    channel: String,
    subscriptions: Arc<Mutex<HashSet<String>>>,
    update_tx: mpsc::Sender<HashSet<String>>,
    update_rx: mpsc::Receiver<HashSet<String>>,
    zmq_tx: mpsc::Sender<Vec<u8>>
}