
## [0.4.0]
- Added 'multithread' feature for running naia-client-socket in a multithreaded environment
- Moved completely off of tokio & in the demos too (prefer smol & runtime-agnostic where necessary)
- Update to use webrtc-unreliable version 0.5.0

--- sorry for skipped changelog here ---

## [0.1.1]
- [Slyklaw](https://github.com/Slyklaw) fixed a Windows incompatibility regarding the lookup of the host's ip address
- Socket initialization now rightly takes a SocketAddr instead of a &str .. duh ..

## [0.1.0]
- Initial release