#protoc --prost_out=src/generated proto/picam3.proto; mv src/generated/_ src/generated/picam3.rs
#protoc --prost_out=src/generated proto/ahqcam.proto; mv src/generated/_ src/generated/ahqcam.rs
protoc --prost_out=src/generated proto/sessions.proto; mv src/generated/_ src/generated/sessions.rs
#protoc --prost_out=src/generated proto/settings.proto; mv src/generated/_ src/generated/settings.rs
protoc --prost_out=src/generated proto/protocol.proto; mv src/generated/_ src/generated/protocol.rs
protoc --prost_out=src/generated proto/control.proto; mv src/generated/_ src/generated/control.rs
