//! The catalogue: every component MCF knows how to build, as data.
//!
//! **Why this is here and not beside the builder.** What MCF can provision is
//! read by more than the thing that builds it: the command line lists it, the
//! daemon is asked what this machine is holding, and a surface draws it. The
//! builder lives in `mcf-cli` because building is a command; the catalogue
//! lives here because a second copy of a pinned digest is a second thing to
//! forget to change (§3.4).
//!
//! Nothing in this module builds anything. It is names, pinned digests, and
//! the sentences that say what having each component lets MCF claim.

/// How a base image installs and reports its packages.
///
/// The recipe used to say `dnf` and `rpm` outright, which was true of the one
/// image there was. The CUDA toolkit ships on Ubuntu, and a second component
/// made the assumption visible by failing on it — `dnf: command not found`,
/// exit 127, before a single file was compiled (F128).
#[derive(Debug, Clone, Copy)]
pub enum Packaging {
    /// Fedora and its relatives.
    Dnf,
    /// Debian and its relatives, which is what the CUDA images are built on.
    Apt,
}

/// One component MCF knows how to provision.
#[derive(Debug, Clone, Copy)]
pub struct Component {
    /// The name the operator types.
    pub name: &'static str,
    /// What having it lets MCF claim.
    pub role: &'static str,
    /// The base image, pinned by digest — the tag beside it is for a reader.
    pub image: &'static str,
    /// That image's digest, which is what is actually pulled.
    pub image_digest: &'static str,
    /// Where the source comes from.
    pub source: &'static str,
    /// Exactly which of it: a commit, never a branch.
    pub commit: &'static str,
    /// The packages the build needs, installed inside the container and
    /// recorded with their exact versions.
    pub packages: &'static [&'static str],
    /// How this image installs them.
    pub packaging: Packaging,
    /// How it is configured.
    pub configure: &'static [&'static str],
    /// What is built out of it, and therefore what MCF may then call.
    pub targets: &'static [&'static str],
}

/// Everything MCF can provision.
pub const COMPONENTS: &[Component] = &[
    Component {
        name: "llama.cpp",
        role: "the reference implementation MCF's own engine is checked against (B-368): \
           tokenizers compared exactly, generations at a measured margin, embeddings \
           at a measured floor",
        // **The base image is chosen for its glibc, not its freshness.** This
        // component's targets are EXECUTABLES that run on the host, so the
        // image's glibc is a floor every one of them carries: built on
        // Fedora 44 (glibc 2.43) they refused to start on a host with 2.41,
        // reporting `version GLIBC_2.43 not found` at the first generation
        // rather than at provision time. `BUILD_SHARED_LIBS=OFF` below was the
        // same lesson learned about llama.cpp's own libraries (F31); glibc is
        // the one library that setting cannot make static.
        //
        // Debian bookworm publishes glibc 2.36, which is older than the hosts
        // MCF runs on, and a binary linked against an older glibc runs against
        // a newer one. Newer is not better here: an image is a floor, and a
        // floor is chosen low.
        image: "docker.io/library/debian:bookworm",
        image_digest: "sha256:2f65600e1252c5649d2213e1d1ea4d74253d26514dc6530102a875e429245929",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "925e1179947ea0c0ebfb0032df18af3a729822be",
        // `ca-certificates` because Debian's minimal image ships none, and
        // without them the clone of a `https://` source fails verification
        // before a line is compiled. The Fedora image carried them, so this
        // was invisible until the base image changed.
        packages: &["build-essential", "cmake", "git", "ca-certificates"],
        packaging: Packaging::Apt,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            // Portable rather than tuned: a provisioned binary is a condition of
            // measurements, and `-march=native` would make it a condition nobody
            // can restate on another machine (§3.4).
            "-DGGML_NATIVE=OFF",
            // Self-contained, because the artifact outlives the container that
            // built it. A shared build bakes the *container's* library path into
            // every binary — `/work/build/bin`, a directory that exists nowhere on
            // the host — so the first provisioned oracle loaded nothing without an
            // incantation (F31). What is provisioned must run where it lands.
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
        ],
        // The server is the one reference tool that exposes the model's
        // distribution — `n_probs` on its completion endpoint — which is what a
        // comparison of logits rather than texts needs (B-373). Nothing else in
        // the reference prints a logit.
        // `llama-mtmd-cli` is the one tool here that takes an image. A model
        // with a vision projector is a model MCF could not put a question to
        // at all, so *vision* was declined as a modality — not because the
        // model lacks it, but because nothing in this build could ask, and a
        // probe reporting *no vision* on that basis would be reporting MCF's
        // own reach as a property of the model (A21, B-320).
        targets: &[
            "llama-tokenize",
            "llama-completion",
            "llama-embedding",
            "llama-server",
            "llama-mtmd-cli",
        ],
    },
    Component {
        name: "llama.cpp-vulkan",
        role: "the same reference, built with a Vulkan back end, so a measurement can be \
           taken on a card that CUDA does not drive — a Radeon, or the graphics on a \
           processor that carves its memory out of the system's. Without it such a \
           card is present and every timing on the machine is a processor timing",
        // **A higher floor than the plain build, and stated.** The Vulkan
        // back end at this commit is written against the loader's headers
        // at 1.3.268 or later — `VK_EXT_layer_settings` — and bookworm ships
        // 1.3.239, which does not compile it. Trixie ships 1.4.309, glslc
        // 2025.2 and glibc 2.41: the binaries built here run on a host with
        // glibc 2.41 or newer, which is a narrower promise than the plain
        // build's 2.36, and it is this component's promise alone. Vulkan
        // adds the loader's headers, the shader compiler and the SPIR-V
        // headers the back end's cmake asks for by name at build time; at
        // run time the binary finds the host's own loader and the driver the
        // host installed for its card, which is what the hardware profiler
        // reports as the runtime.
        image: "docker.io/library/debian:trixie",
        image_digest: "sha256:6788062a1b42ac281f053ac876170b79a3eaed5d61383b8ed7eaca6c6965f3b1",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "925e1179947ea0c0ebfb0032df18af3a729822be",
        packages: &[
            "build-essential",
            "cmake",
            "git",
            "ca-certificates",
            "libvulkan-dev",
            "glslc",
            "spirv-headers",
        ],
        packaging: Packaging::Apt,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DGGML_NATIVE=OFF",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
            "-DGGML_VULKAN=ON",
        ],
        targets: &[
            "llama-tokenize",
            "llama-completion",
            "llama-embedding",
            "llama-server",
            "llama-mtmd-cli",
        ],
    },
    Component {
        name: "llama.cpp-cuda",
        role: "the same reference, built with a CUDA backend, so a measurement can \
           be taken on the GPU as well as the CPU. Without it MCF's engine \
           reports no devices and every timing on this machine is a CPU timing \
           whether or not a card is installed (F127, F128)",
        // A CUDA toolkit image, because nvcc is what the backend needs and the
        // Fedora image beside this one carries none. Compiling needs the toolkit;
        // it does not need a GPU, so this build is as reproducible as the other.
        image: "docker.io/nvidia/cuda:12.9.1-devel-ubuntu24.04",
        image_digest: "sha256:020bc241a628776338f4d4053fed4c38f6f7f3d7eb5919fecb8de313bb8ba47c",
        source: "https://github.com/ggml-org/llama.cpp.git",
        // The SAME commit as the CPU build. Two backends of one source are
        // comparable; two backends of two sources are not, and putting one against
        // the other is the whole point of having both.
        commit: "925e1179947ea0c0ebfb0032df18af3a729822be",
        packages: &["build-essential", "cmake", "git"],
        packaging: Packaging::Apt,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DGGML_NATIVE=OFF",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
            "-DGGML_CUDA=ON",
            // The architectures compiled for are a condition of the artifact, the
            // way `-march` would be, so they are stated rather than left to the
            // toolkit's default. 89 is Ada, 120 is Blackwell — the card here is the
            // latter, and a binary that ran only here would be one nobody could
            // restate a measurement with (§3.4).
            "-DCMAKE_CUDA_ARCHITECTURES=89;120",
            // Static, for the same reason `BUILD_SHARED_LIBS=OFF` is: what is
            // provisioned must run where it lands (F31). The first CUDA build
            // linked the container's libcudart.so.12 and would not start on this
            // host, which carries CUDA 13. cudart alone was not enough — cuBLAS and
            // NCCL were still dynamic, and NCCL is for spreading one model across
            // several cards, which this is not doing. The driver library is the one
            // thing that must come from the machine, and it does.
            "-DCMAKE_CUDA_RUNTIME_LIBRARY=Static",
            "-DGGML_STATIC=ON",
            "-DGGML_CUDA_NCCL=OFF",
        ],
        // The same tools as the processor build, including the one that
        // takes an image: two backends of one source are comparable only if
        // they can be asked the same questions.
        targets: &[
            "llama-tokenize",
            "llama-completion",
            "llama-embedding",
            "llama-server",
            "llama-mtmd-cli",
        ],
    },
    Component {
        name: "SDL3",
        role: "a window, keyboard and mouse events, and a 2D renderer for the desktop \
               application (B-405). It is the ONLY thing vendored for it: MCF draws every \
               panel, table and button itself, with the layout the terminal console already \
               uses, so no widget toolkit is admitted and no font library is needed — SDL \
               carries an 8x8 font of its own",
        image: "registry.fedoraproject.org/fedora:44",
        image_digest: "sha256:5a4a491c33973b8173e6134d6f00e77f27cebef581c9b34420b2b6183a6398df",
        source: "https://github.com/libsdl-org/SDL.git",
        // release-3.4.14.
        commit: "147a8ee32dbf9ac02f3794964490687b6bbda1bc",
        packages: &[
            "gcc",
            "cmake",
            "git",
            "make",
            // The windowing systems SDL talks to. Loaded at runtime rather than
            // linked, so the built library runs on a machine with either.
            "libX11-devel",
            "libXext-devel",
            "libXrandr-devel",
            "libXcursor-devel",
            "libXfixes-devel",
            "libXi-devel",
            "libXScrnSaver-devel",
            "libXtst-devel",
            "libxkbcommon-devel",
            "wayland-devel",
            "wayland-protocols-devel",
            "mesa-libGL-devel",
            "mesa-libEGL-devel",
        ],
        packaging: Packaging::Dnf,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DSDL_SHARED=OFF",
            // Static, for the same reason every other provisioned artifact is:
            // what is provisioned must run where it lands (F31).
            "-DSDL_STATIC=ON",
            // Position-independent, because what links it is a Rust binary and
            // Rust links a position-independent executable. Without this the
            // archive builds, and then the link fails on a relocation nobody
            // reading the recipe would have predicted.
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            // Everything below is off because MCF does not use it — and because
            // each one is a part of the tree that is NOT zlib. Switching them
            // off is not tidiness: it is what makes the shipped tree almost
            // entirely one licence, and the finding in vendored.md rests on
            // this exact list.
            //
            //   HIDAPI   tri-licensed, one option being GPL-3.0
            //   VULKAN   pulls a Khronos header under Apache-2.0
            //   OPENVR   Valve's, BSD-3-Clause
            //   TESTS    public domain, and not shipped anyway
            "-DSDL_HIDAPI=OFF",
            "-DSDL_HIDAPI_JOYSTICK=OFF",
            "-DSDL_VULKAN=OFF",
            "-DSDL_RENDER_VULKAN=OFF",
            "-DSDL_OPENVR=OFF",
            "-DSDL_TESTS=OFF",
            "-DSDL_EXAMPLES=OFF",
            // Subsystems a measuring instrument has no use for. Less code is
            // less to verify and less to go wrong.
            "-DSDL_AUDIO=OFF",
            "-DSDL_CAMERA=OFF",
            "-DSDL_HAPTIC=OFF",
            "-DSDL_JOYSTICK=OFF",
            "-DSDL_SENSOR=OFF",
            "-DSDL_POWER=OFF",
        ],
        targets: &["SDL3-static"],
    },
];
