# IO

Io should be done asynchronous using tokios async `tokio::fs`. Additionally there is an `IoProtocol`
to deligate IO work to, which responds with the read file contents via a oneshot channel.
