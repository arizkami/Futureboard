# ─────────────────────────────────────────────────────────────────────────────
#  Mochi DAW · Native shell (apps/native)
#
#  Minimal borderless window built on the SphereEngine framework, used to
#  validate that the native runtime can host the DAW's outer shell — top
#  transport bar, browser column, arrangement, mixer, inspector, status bar —
#  before any real DAW behaviour is ported across.
#
#  Build paths:
#
#  • Standalone — configure this directory directly:
#        cmake -S apps/native -B build/native
#        cmake --build build/native --config Release
#    This will add ../../frameworks/SphereEngine as a sub-build automatically.
#
#  • Inside another tree — add this directory from a parent CMakeLists that
#    already provides the SphereKit_GraphicComponent target.  The
#    add_subdirectory(SphereEngine ...) call below becomes a no-op in that
#    case because the target already exists.
# ─────────────────────────────────────────────────────────────────────────────

cmake_minimum_required(VERSION 3.20)

project(MochiDAWNative LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Mirror SphereEngine's MSVC runtime selection so we link cleanly against it.
if(MSVC)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
    set(CMAKE_POLICY_DEFAULT_CMP0091 NEW)
endif()

# Bring in SphereEngine only if it has not already been added by a parent.
set(MOCHI_DAW_SPHERE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../frameworks/SphereEngine"
    CACHE PATH "Path to the SphereEngine source tree.")

if(NOT TARGET SphereKit_GraphicComponent)
    if(NOT EXISTS "${MOCHI_DAW_SPHERE_ROOT}/CMakeLists.txt")
        message(FATAL_ERROR
            "SphereEngine not found at ${MOCHI_DAW_SPHERE_ROOT}.\n"
            "Set MOCHI_DAW_SPHERE_ROOT or include SphereEngine from a parent CMakeLists.")
    endif()
    add_subdirectory(${MOCHI_DAW_SPHERE_ROOT} ${CMAKE_BINARY_DIR}/SphereEngine)
endif()

# ── Per-app resource generation ─────────────────────────────────────────────
#
# The framework's CMakeLists already runs scripts/gen_resources.ts on its own
# example/shared/resources/resrouces.xml with the default init function name
# `InitBinaryResources` (which emits a self-installing global constructor and
# ends up inside SphereKit.Foundation.dll).  Re-using that exact name here
# would produce duplicate symbols, so we run a SECOND generator pass against
# THIS app's resources.xml with a custom init function name and call it
# explicitly from main().
#
# Path scheme: <file>fonts/Inter-Regular.ttf</file> in apps/native/shared/
# resources.xml becomes res://fonts/Inter-Regular.ttf at runtime.

set(MOCHI_DAW_RESOURCES_XML "${CMAKE_CURRENT_SOURCE_DIR}/shared/resources.xml")
set(MOCHI_DAW_RESOURCE_GENERATOR "${MOCHI_DAW_SPHERE_ROOT}/scripts/gen_resources.ts")
set(MOCHI_DAW_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated")
set(MOCHI_DAW_GENERATED_H   "${MOCHI_DAW_GENERATED_DIR}/MochiResources.hpp")
set(MOCHI_DAW_GENERATED_CPP "${MOCHI_DAW_GENERATED_DIR}/MochiResources.cpp")

# Re-use the same Bun the framework picked up.  If we're configured standalone
# (parent has not exposed BUN_EXECUTABLE), look it up the same way as the
# framework's own CMakeLists does.
if(NOT BUN_EXECUTABLE)
    find_program(BUN_EXECUTABLE
        NAMES bun bun.exe
        PATHS
            "$ENV{USERPROFILE}/.bun/bin"
            "$ENV{HOME}/.bun/bin"
    )
    if(NOT BUN_EXECUTABLE)
        message(FATAL_ERROR
            "Bun is required to embed apps/native/shared/resources.xml but was not found.")
    endif()
endif()

add_custom_command(
    OUTPUT  ${MOCHI_DAW_GENERATED_H} ${MOCHI_DAW_GENERATED_CPP}
    COMMAND ${BUN_EXECUTABLE} run
            ${MOCHI_DAW_RESOURCE_GENERATOR}
            ${MOCHI_DAW_RESOURCES_XML}
            ${MOCHI_DAW_GENERATED_H}
            ${MOCHI_DAW_GENERATED_CPP}
            InitMochiResources
    DEPENDS ${MOCHI_DAW_RESOURCES_XML} ${MOCHI_DAW_RESOURCE_GENERATOR}
    COMMENT "Generating MochiDAW binary resources from ${MOCHI_DAW_RESOURCES_XML}"
)

# ── Executable ───────────────────────────────────────────────────────────────
set(MOCHI_DAW_SOURCES
    src/main.cpp
    ${MOCHI_DAW_GENERATED_CPP}
    ${MOCHI_DAW_GENERATED_H}
)

if(WIN32)
    add_executable(MochiDAW WIN32 ${MOCHI_DAW_SOURCES})
else()
    # Non-Windows is not wired yet (SphereEngine only ships a Win32 backend);
    # keep the target buildable so static analysis tools still see the sources.
    add_executable(MochiDAW ${MOCHI_DAW_SOURCES})
endif()

# Public framework include surface lives in <SphereEngine>/include, but the
# framework itself relies on a handful of directory-scope `include_directories`
# (Skia root, Yoga, RtAudio, nlohmann-json) that do NOT propagate across
# `add_subdirectory`.  SphereEngine headers like `core/events/SkiaEvent.hpp`
# transitively `#include <include/core/SkRect.h>`, which only resolves when the
# Skia prebuilt root is on the include path — so we replicate the same list
# here as PRIVATE includes on the MochiDAW target.
#
# SKIA_DIR is a CACHE PATH set by SphereEngine's CMakeLists during its
# configure step; it's available in our scope after add_subdirectory(...).
target_include_directories(MochiDAW PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${MOCHI_DAW_SPHERE_ROOT}/include
    ${MOCHI_DAW_SPHERE_ROOT}/external/yoga
    ${MOCHI_DAW_SPHERE_ROOT}/external/rtaudio
    ${MOCHI_DAW_SPHERE_ROOT}/external/nlohmann-json
    ${SKIA_DIR}
    ${SKIA_DIR}/include
    ${CMAKE_BINARY_DIR}/SphereEngine/generated   # framework's InitBinaryResources
    ${MOCHI_DAW_GENERATED_DIR}                   # our InitMochiResources
)

# Inherit the same Skia + Win32 compile definitions the framework uses, so
# headers gated on SK_D3D / SK_DIRECTWRITE / NOMINMAX / WIN32_LEAN_AND_MEAN
# compile the same way against us as against the framework's own TUs.
target_compile_definitions(MochiDAW PRIVATE
    SKIA_STATIC
    SK_RELEASE
    SK_DISABLE_LEGACY_PATH_METHODS
)
if(WIN32)
    target_compile_definitions(MochiDAW PRIVATE
        SK_D3D
        SK_DIRECTWRITE
        NOMINMAX
        WIN32_LEAN_AND_MEAN
    )
endif()

target_link_libraries(MochiDAW PRIVATE
    SphereKit_GraphicComponent
)

# Stage assets next to the binary so runtime lookups (fonts, future icons)
# resolve without an absolute path.
add_custom_command(TARGET MochiDAW POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_CURRENT_SOURCE_DIR}/shared
        $<TARGET_FILE_DIR:MochiDAW>/shared
    COMMENT "Stage apps/native/shared -> $<TARGET_FILE_DIR:MochiDAW>/shared"
)

# Mirror SphereEngine's example convention — copy the exe to the top-level
# build directory so it can find sibling V8/Skia runtime DLLs that the
# framework stages there.
add_custom_command(TARGET MochiDAW POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:MochiDAW>
        ${CMAKE_BINARY_DIR}/$<TARGET_FILE_NAME:MochiDAW>
    COMMENT "Copy MochiDAW -> ${CMAKE_BINARY_DIR}"
)
