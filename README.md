# cqueue

An extremely simple in-memory job queue implementation.

Supported transports: **HTTP**<br>
Supported storages: **In-Memory**

# TODO;

* [x] Implement in-memory queue store
* [ ] Add HTTP server capabilities
* [ ] Add docker support
* [ ] Observability/Monitoring solutions
* [ ] OpenAPI/Swagger support?
* [ ] Implement CI/CD pipelines using GitHub actions
* [ ] Add command line arg parser to configure app properties. E.g, server port.
* [ ] Improve documentation and add examples
* [ ] Changelog generator?
* [ ] Auth support

# Developer notes

* I know the requirement says consumer provides ID in headers, but I thought it isn't much fun. Also, the consumer may
  not know any ID, as we return ID to producer only.
* As this is an in-memory store and everything will purged after restart, I don't really care about keeping the ID
  sequence on app startup.
* At first, I was keeping AtomicUsize for ID counter, then I changed it to actual size of the `jobs` map.
* The fun part is, we may discuss the "ID thing" for quite while. Sharding/Partitioning, Replication, LB? cool
  stuff!!
