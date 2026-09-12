#[derive(Debug, Clone, Copy)]
pub enum Packaging {
    Dnf,
    Apt,
}

#[derive(Debug, Clone, Copy)]
pub struct Component {
    pub name: &'static str,
    pub role: &'static str,
    pub image: &'static str,
    pub image_digest: &'static str,
    pub source: &'static str,
    pub commit: &'static str,
    pub packages: &'static [&'static str],
    pub packaging: Packaging,
    pub configure: &'static [&'static str],
    pub targets: &'static [&'static str],
}

pub const COMPONENTS: &[Component] = &[
    Component {
        name: "llama.cpp",
        role: "the reference implementation MCF's own engine is checked against (B-368): \
           tokenizers compared exactly, generations at a measured margin, embeddings \
           at a measured floor",
        image: "docker.io/library/debian:bookworm",
        image_digest: "sha256:2f65600e1252c5649d2213e1d1ea4d74253d26514dc6530102a875e429245929",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "acecd56032ddc34bada14a2d978f110d9c987095",
        packages: &["build-essential", "cmake", "git", "ca-certificates"],
        packaging: Packaging::Apt,
        configure: &[
            "-DCMAKE_BUILD_TYPE=Release",
            "-DGGML_NATIVE=OFF",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DLLAMA_CURL=OFF",
            "-DLLAMA_BUILD_TESTS=OFF",
            "-DLLAMA_BUILD_EXAMPLES=ON",
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
        name: "llama.cpp-vulkan",
        role: "the same reference, built with a Vulkan back end, so a measurement can be \
           taken on a card that CUDA does not drive — a Radeon, or the graphics on a \
           processor that carves its memory out of the system's. Without it such a \
           card is present and every timing on the machine is a processor timing",
        image: "docker.io/library/debian:trixie",
        image_digest: "sha256:6788062a1b42ac281f053ac876170b79a3eaed5d61383b8ed7eaca6c6965f3b1",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "acecd56032ddc34bada14a2d978f110d9c987095",
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
        image: "docker.io/nvidia/cuda:12.9.1-devel-ubuntu24.04",
        image_digest: "sha256:020bc241a628776338f4d4053fed4c38f6f7f3d7eb5919fecb8de313bb8ba47c",
        source: "https://github.com/ggml-org/llama.cpp.git",
        commit: "acecd56032ddc34bada14a2d978f110d9c987095",
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
            "-DCMAKE_CUDA_ARCHITECTURES=89;120",
            "-DCMAKE_CUDA_RUNTIME_LIBRARY=Static",
            "-DGGML_STATIC=ON",
            "-DGGML_CUDA_NCCL=OFF",
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
        name: "SDL3",
        role: "a window, keyboard and mouse events, and a 2D renderer for the desktop \
               application (B-405). It is the ONLY thing vendored for it: MCF draws every \
               panel, table and button itself, with the layout the terminal console already \
               uses, so no widget toolkit is admitted and no font library is needed — SDL \
               carries an 8x8 font of its own",
        image: "registry.fedoraproject.org/fedora:44",
        image_digest: "sha256:5a4a491c33973b8173e6134d6f00e77f27cebef581c9b34420b2b6183a6398df",
        source: "https://github.com/libsdl-org/SDL.git",
        commit: "147a8ee32dbf9ac02f3794964490687b6bbda1bc",
        packages: &[
            "gcc",
            "cmake",
            "git",
            "make",
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
            "-DSDL_STATIC=ON",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
            "-DSDL_HIDAPI=OFF",
            "-DSDL_HIDAPI_JOYSTICK=OFF",
            "-DSDL_VULKAN=OFF",
            "-DSDL_RENDER_VULKAN=OFF",
            "-DSDL_OPENVR=OFF",
            "-DSDL_TESTS=OFF",
            "-DSDL_EXAMPLES=OFF",
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

#[cfg(test)]
mod tests;
