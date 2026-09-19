cmake_minimum_required(VERSION 3.15)
project(rrc_cpp VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find or build the Rust library
# In a typical setup, `cargo build --release` produces `librrc_cpp.a` or `librrc_cpp.so`
# in `../../target/release/`
set(RRC_RUST_TARGET_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release")

add_library(rrc_static STATIC IMPORTED)
set_target_properties(rrc_static PROPERTIES
    IMPORTED_LOCATION "${RRC_RUST_TARGET_DIR}/librrc_cpp.a"
    INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/include"
)

# Header-only interface target for C++ wrapper
add_library(rrc_cpp INTERFACE)
target_include_directories(rrc_cpp INTERFACE "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(rrc_cpp INTERFACE rrc_static pthread dl m)

# Optional example executable
if(BUILD_EXAMPLES)
    add_executable(rrc_example examples/example.cpp)
    target_link_libraries(rrc_example PRIVATE rrc_cpp)
endif()
