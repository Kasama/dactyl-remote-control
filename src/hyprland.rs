use futures::{Stream, StreamExt};
use tokio::net::{UnixSocket, UnixStream};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, FramedRead, LinesCodec};

pub struct Hyprland {
    inner: FramedRead<UnixStream, EventDecoder>,
}

#[derive(Debug)]
pub enum Event {
    Nothing(String),
    ActiveWindow { class: String, title: String },
}

impl Event {
    pub fn from_line(line: &str) -> Result<Self, anyhow::Error> {
        let (event_name, data) = line
            .split_once(">>")
            .ok_or_else(|| anyhow::anyhow!("Invalid event line: {}", line))?;
        match event_name.trim() {
            "activewindow" => {
                let (class, title) = data
                    .split_once(',')
                    .ok_or_else(|| anyhow::anyhow!("Invalid active_window data: {}", data))?;
                Ok(Event::ActiveWindow {
                    class: class.trim().to_string(),
                    title: title.trim().to_string(),
                })
            }
            _ => Ok(Event::Nothing(line.to_string()))
        }
    }
}

struct EventDecoder {
    inner: LinesCodec,
}

impl EventDecoder {
    fn new() -> Self {
        Self {
            inner: LinesCodec::new(),
        }
    }
}

impl Decoder for EventDecoder {
    type Item = Event;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(line) = self.inner.decode(src)? {
            Event::from_line(&line).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Hyprland {
    pub async fn connect(hyprland_signature: &str) -> Result<Self, anyhow::Error> {
        let socket = UnixSocket::new_stream()?;
        let socket_path = format!(
            "{}/hypr/{}/.socket2.sock",
            std::env::var("XDG_RUNTIME_DIR")?,
            hyprland_signature
        );
        let stream = socket.connect(socket_path).await?;
        let reader = FramedRead::new(stream, EventDecoder::new());
        Ok(Hyprland { inner: reader })
    }
}

impl Stream for Hyprland {
    type Item = Result<Event, anyhow::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}
