use crate::network::StateNetwork;
use discv5::TalkRequest;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, warn, Instrument};
use trin_core::{portalnet::types::messages::Message, utp::stream::UtpListenerEvent};

pub struct StateEvents {
    pub network: Arc<StateNetwork>,
    pub event_rx: UnboundedReceiver<TalkRequest>,
    pub utp_listener_rx: UnboundedReceiver<UtpListenerEvent>,
}

impl StateEvents {
    pub async fn start(mut self) {
        loop {
            tokio::select! {
                Some(talk_request) = self.event_rx.recv() => {
                    self.handle_state_talk_request(talk_request);
                }
                Some(event) = self.utp_listener_rx.recv() => {
                    if let Err(err) = self.network.overlay.process_utp_event(event) {
                        warn!("Error processing uTP payload: {err}");
                    }
                }
            }
        }
    }

    /// Handle state network TalkRequest event
    fn handle_state_talk_request(&self, talk_request: TalkRequest) {
        let network = Arc::clone(&self.network);
        tokio::spawn(async move {
            let reply = match network
                .overlay
                .process_one_request(&talk_request)
                .instrument(tracing::info_span!("state_network"))
                .await
            {
                Ok(response) => {
                    debug!("Sending reply: {:?}", response);
                    Message::from(response).into()
                }
                Err(error) => {
                    error!("Failed to process portal state event: {error}");
                    error.to_string().into_bytes()
                }
            };
            if let Err(error) = talk_request.respond(reply) {
                warn!("Failed to send reply: {error}");
            }
        });
    }
}
