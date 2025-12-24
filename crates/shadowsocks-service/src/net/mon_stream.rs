//! TCP stream with flow statistic monitored

use std::{
    io::{self, IoSlice},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use pin_project::pin_project;
use shadowsocks::relay::{
    Address,
    tcprelay::{GetUser, ProxyServerStream},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::flow::FlowStat;

/// Monitored `ProxyStream`
#[pin_project]
pub struct MonProxyStream<S> {
    #[pin]
    stream: ProxyServerStream<S>,
    flow_stat: Arc<FlowStat>,
}

impl<S> MonProxyStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    pub fn from_stream(stream: ProxyServerStream<S>, flow_stat: Arc<FlowStat>) -> Self {
        Self { stream, flow_stat }
    }

    #[inline]
    pub fn get_ref(&self) -> &ProxyServerStream<S> {
        &self.stream
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut ProxyServerStream<S> {
        &mut self.stream
    }

    #[inline]
    pub fn into_inner(self) -> ProxyServerStream<S> {
        self.stream
    }

    #[inline]
    pub async fn handshake(&mut self) -> io::Result<Address> {
        self.stream.handshake().await
    }
}

impl<S> AsyncRead for MonProxyStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut this = self.project();
        match this.stream.as_mut().poll_read(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(())) => {
                let n = buf.filled().len();
                this.flow_stat.incr_rx(n as u64, this.stream.user().as_deref());
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
        }
    }
}

impl<S> AsyncWrite for MonProxyStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut this = self.project();
        match this.stream.as_mut().poll_write(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(n)) => {
                this.flow_stat.incr_tx(n as u64, this.stream.user().as_deref());
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().stream.poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().stream.poll_shutdown(cx)
    }

    #[inline]
    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.project().stream.poll_write_vectored(cx, bufs)
    }
}
