use futures::StreamExt;
use tokio_i3ipc::event::WindowData;
use tokio_i3ipc::reply::Node;

#[async_trait::async_trait]
pub trait I3Ext {
    async fn find_focused_node(&mut self) -> Result<Node, anyhow::Error>;
    async fn subscribe_to_window_focus_events<F>(mut self, f: F) -> Result<(), anyhow::Error>
    where
        F: Fn(Option<WindowData>, WindowData) -> Result<(), anyhow::Error> + Send;
}

pub trait I3NodeWalker {
    fn find_focused_node(self) -> Result<Node, anyhow::Error>;
}

#[async_trait::async_trait]
impl I3Ext for tokio_i3ipc::I3 {
    async fn find_focused_node(&mut self) -> Result<Node, anyhow::Error> {
        let tree = self.get_tree().await?;

        tree.find_focused_node()
    }

    async fn subscribe_to_window_focus_events<F>(mut self, f: F) -> Result<(), anyhow::Error>
    where
        F: Fn(Option<WindowData>, WindowData) -> Result<(), anyhow::Error> + Send,
    {
        let subscription = self
            .subscribe([
                tokio_i3ipc::event::Subscribe::Window,
                tokio_i3ipc::event::Subscribe::Shutdown,
            ])
            .await?;
        if !subscription.success {
            eprintln!("Failed to subscribe to i3 events: {:?}", subscription.error);
            return Err(anyhow::anyhow!("Failed to subscribe to i3 events"));
        }

        let mut previous_ev: Option<WindowData> = None;

        let mut listener = self.listen();
        while let Some(e) = listener.next().await {
            match e? {
                tokio_i3ipc::event::Event::Window(ev) => {
                    if let tokio_i3ipc::event::WindowChange::Focus = ev.change {
                        let p_ev = Some(*ev.clone());
                        f(previous_ev, *ev)?;
                        previous_ev = p_ev;
                    }
                }
                tokio_i3ipc::event::Event::Shutdown(ev) => println!("shutdown: {:?}", ev),
                _ => unreachable!("unexpected not subscribed event"),
            }
        }
        Ok(())
    }
}

impl I3NodeWalker for tokio_i3ipc::reply::Node {
    fn find_focused_node(self) -> Result<Node, anyhow::Error> {
        let mut node = self;

        while !node.focused {
            let focused_node_id = node
                .focus
                .first()
                .ok_or_else(|| anyhow::anyhow!("no focused node"))?;

            node = node
                .nodes
                .into_iter()
                .find(|n| n.id == *focused_node_id)
                .ok_or_else(|| anyhow::anyhow!("focused node id"))?;
        }

        Ok(node)
    }
}
