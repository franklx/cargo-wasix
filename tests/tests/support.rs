use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

pub fn root() -> PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    std::thread_local! {
        static TEST_ID: usize = NEXT_ID.fetch_add(1, SeqCst);
    }
    let id = TEST_ID.with(|n| *n);
    let mut path = env::current_exe().unwrap();
    path.pop(); // chop off exe name
    path.pop(); // chop off 'deps'
    path.pop(); // chop off 'debug'
    path.push("tests");
    fs::create_dir_all(&path).unwrap();
    path.join(&format!("t{}", id))
}

pub fn project() -> ProjectBuilder {
    ProjectBuilder::new(root())
}

pub struct Project {
    root: PathBuf,
    runtime_override: Option<String>,
}

pub struct ProjectBuilder {
    project: Project,
    saw_manifest: bool,
}

impl ProjectBuilder {
    pub fn new(root: PathBuf) -> ProjectBuilder {
        println!(" ============ {} =============== ", root.display());
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).unwrap();
        ProjectBuilder {
            project: Project {
                root,
                runtime_override: None,
            },
            saw_manifest: false,
        }
    }

    pub fn root(&self) -> PathBuf {
        self.project.root()
    }

    pub fn file<B: AsRef<Path>>(&mut self, path: B, body: &str) -> &mut Self {
        self._file(path.as_ref(), body);
        self
    }

    pub fn override_runtime(&mut self, runtime_override: &str) -> &mut Self {
        self.project.runtime_override = Some(runtime_override.to_string());
        self
    }

    fn _file(&mut self, path: &Path, body: &str) {
        if path == Path::new("Cargo.toml") {
            self.saw_manifest = true;
        }
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(self.root().join(path), body).unwrap();
    }

    pub fn build(&mut self) -> Project {
        if !self.saw_manifest {
            self.file(
                "Cargo.toml",
                r#"
                    [package]
                    name = "foo"
                    version = "1.0.0"
                "#,
            );
        }
        Project {
            root: self.project.root.clone(),
            runtime_override: self.project.runtime_override.clone(),
        }
    }
}

impl Project {
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    pub fn build_dir(&self) -> PathBuf {
        self.root().join("target")
    }

    pub fn debug_wasm(&self, name: &str) -> PathBuf {
        self.build_dir()
            .join("wasm32-wasmer-wasi")
            .join("debug")
            .join(format!("{}.wasm", name))
    }

    pub fn release_wasm(&self, name: &str) -> PathBuf {
        self.build_dir()
            .join("wasm32-wasmer-wasi")
            .join("release")
            .join(format!("{}.wasm", name))
    }

    pub fn cargo_wasix(&self, cmd: &str) -> Command {
        let mut process = super::cargo_wasix(cmd);
        process
            .current_dir(&self.root)
            .env("CARGO_HOME", self.root.join("cargo-home"));

        if let Some(runtime_override) = &self.runtime_override {
            process.env("CARGO_TARGET_WASM32_WASMER_WASI_RUNNER", runtime_override);
        }

        process.arg("--color=never");

        process
    }
}
