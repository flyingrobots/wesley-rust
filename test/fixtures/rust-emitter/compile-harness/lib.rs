// The crate the compile tests build: generated models and codecs side by side,
// with a minimal LE-binary runtime in `crate::codec`.

include!("models.rs");
include!("codecs.rs");

pub mod codec {
    include!("codec_runtime.rs");
}
