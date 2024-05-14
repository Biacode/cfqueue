# cqueue

An extremely simple in-memory job queue implementation.

Supported transports: **HTTP**<br>
Supported storages: **In-Memory**

# Getting started

* Install [Rust toolchain](https://www.rust-lang.org/tools/install)
* Clone [cqueue repository](https://github.com/Biacode/cqueue) `git clone git@github.com:Biacode/cqueue.git`
* Execute `cargo run` command from within the git repo `cd cqueue && cargo run`

# TODO;

* [x] Implement in-memory queue store
* [x] Add HTTP server capabilities
* [ ] Add docker support
* [ ] Observability/Monitoring solutions
* [ ] OpenAPI/Swagger support?
* [ ] Implement CI/CD pipelines using GitHub actions
* [ ] Add command line arg parser to configure app properties. E.g, server port
* [ ] The app should be configurable with env variables as well (perhaps with higher priority?)
* [ ] Improve documentation and add examples
* [ ] Changelog generator?
* [ ] Auth support

# Developer notes

* I know the requirement says consumer provides ID in headers, but I thought it isn't much fun. Also, the consumer may
  not know any ID, as we return ID only to producer.
* As this is an in-memory store and everything will purged after restart, I don't really care about keeping the ID
  sequence.
* At first, I was keeping AtomicUsize for ID counter, then I changed it to actual size of the `jobs` map.
* The fun part is, we may discuss the "ID thing" for quite some time. Sharding/Partitioning, Replication, LB? cool
  stuff!!
* To save some time I borrowed an example server impl
  from [Axum examples](https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs)

# API

The queue exposes a REST API that producers and consumers perform HTTP requests against in JSON.

Supported operations:

### `/jobs/enqueue`

Add a job to the queue. The job definition can be found below.
Returns the ID of the job

### `/jobs/dequeue`

Returns a job from the queue
Jobs are considered available for Dequeue if the job has not been concluded and has not dequeued already

### `/jobs/{job_id}/conclude`

Provided an input of a job ID, finish execution on the job and consider it done

### `/jobs/{job_id}`

Given an input of a job ID, get information about a job tracked by the queue

The lifecycle of requests made for a job might look like this:

`/jobs/enqueue -> /jobs/dequeue -> /jobs/{job_id}/conclude`

