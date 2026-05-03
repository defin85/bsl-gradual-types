use zed_extension_api as zed;

struct BslExtension;

impl zed::Extension for BslExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(BslExtension);
