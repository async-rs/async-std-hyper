use std::convert::Infallible;

use async_std::net::TcpListener;
use async_std::task;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

async fn hello(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World!")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    task::block_on(async {
        let addr = "127.0.0.1:3000";
        let listener = TcpListener::bind(addr).await?;

        let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello)) });
        let server = Server::builder(compat::HyperListener(listener))
            .executor(compat::HyperExecutor)
            .serve(make_svc);

        println!("Listening on http://{}", addr);
        server.await?;
        Ok(())
    })
}

pub mod compat {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use async_std::io;
    use async_std::net::{TcpListener, TcpStream};
    use async_std::prelude::*;
    use async_std::task;

    #[derive(Clone)]
    pub struct HyperExecutor;

    impl<F> hyper::rt::Executor<F> for HyperExecutor
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        fn execute(&self, fut: F) {
            task::spawn(fut);
        }
    }

    pub struct HyperListener(pub TcpListener);

    impl hyper::server::accept::Accept for HyperListener {
        type Conn = HyperStream;
        type Error = io::Error;

        fn poll_accept(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
            let stream = task::ready!(Pin::new(&mut self.0.incoming()).poll_next(cx)).unwrap()?;
            Poll::Ready(Some(Ok(HyperStream(stream))))
        }
    }

    pub struct HyperStream(pub TcpStream);

    impl tokio::io::AsyncRead for HyperStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for HyperStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_close(cx)
        }
    }
}
