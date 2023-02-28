use std::path::{Path, PathBuf};
use std::{env, fs, io};

#[derive(Debug, Clone)]
pub struct Builder {
    file_descriptor_set_path: PathBuf,
}

impl Default for Builder {
    fn default() -> Self {
        let file_descriptor_set_path = env::var_os("OUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("file_descriptor_set.bin");

        Self {
            file_descriptor_set_path,
        }
    }
}

impl Builder {
    /// Create a new builder with default parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the path where the encoded file descriptor set is created.
    /// By default, it is created at `$OUT_DIR/file_descriptor_set.bin`.
    ///
    /// This overrides the path specified by
    /// [`prost_build::Config::file_descriptor_set_path`].
    pub fn file_descriptor_set_path<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.file_descriptor_set_path = path.into();
        self
    }

    /// Configure `config` to derive [`prost_extra::MessageExt`] for all messages included in `protos`.
    /// This method does not generate prost-extra compatible code,
    /// but `config` may be used later to compile protocol buffers independently of [`Builder`].
    /// `protos` and `includes` should be the same when [`prost_build::Config::compile_protos`] is called on `config`.
    ///
    /// ```ignore
    /// let mut config = Config::new();
    ///
    /// // Customize config here
    ///
    /// Builder::new()
    ///     .configure(&mut config, &["path/to/protobuf.proto"], &["path/to/include"])
    ///     .expect("Failed to configure for reflection");
    ///
    /// // Custom compilation process with `config`
    /// config.compile_protos(&["path/to/protobuf.proto"], &["path/to/includes"])
    ///     .expect("Failed to compile protocol buffers");
    /// ```
    pub fn configure(
        &mut self,
        config: &mut prost_build::Config,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        config
            .file_descriptor_set_path(&self.file_descriptor_set_path)
            .compile_protos(protos, includes)?;

        let buf = fs::read(&self.file_descriptor_set_path)?;
        let descriptor =
            prost_reflect::DescriptorPool::decode(buf.as_ref()).expect("Invalid file descriptor");

        for message in descriptor.all_messages() {
            let full_name = message.full_name();
            config
                .type_attribute(full_name, "#[derive(::prost_extra::MessageExt)]")
                .type_attribute(
                    full_name,
                    format!(r#"#[prost_extra(message_name = "{}")]"#, full_name,),
                );
        }

        Ok(())
    }

    /// Compile protocol buffers into Rust with given [`prost_build::Config`].
    pub fn compile_protos_with_config(
        &mut self,
        mut config: prost_build::Config,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        self.configure(&mut config, protos, includes)?;

        config.skip_protoc_run().compile_protos(protos, includes)
    }

    /// Compile protocol buffers into Rust.
    pub fn compile_protos(
        &mut self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> io::Result<()> {
        self.compile_protos_with_config(prost_build::Config::new(), protos, includes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let mut config = prost_build::Config::new();
        let mut builder = Builder::new();
        let tmpdir = std::env::temp_dir();
        config.out_dir(tmpdir.clone());

        builder
            .file_descriptor_set_path(tmpdir.join("file_descriptor_set.bin"))
            .compile_protos_with_config(config, &["src/test.proto"], &["src"])
            .unwrap();

        assert!(tmpdir.join("my.test.rs").exists());

        let buf = fs::read_to_string(tmpdir.join("my.test.rs")).unwrap();
        let num_derive = buf
            .lines()
            .filter(|line| line.trim_start() == "#[derive(::prost_extra::MessageExt)]")
            .count();

        assert_eq!(num_derive, 3);
    }
}
