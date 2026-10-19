cmake_minimum_required(VERSION 3.13)
project(runner LANGUAGES CXX)

# Define the application target. To change its name, change BINARY_NAME in the
# top-level CMakeLists.txt, not the value here, or `flutter run` will no longer
# work.
#
# Any new source files that you add to the application should be added here.
add_executable(${BINARY_NAME}
  "main.cc"
  "my_application.cc"
  "broker_process_channel.cc"
  "${FLUTTER_MANAGED_DIR}/generated_plugin_registrant.cc"
)

# Apply the standard set of build settings. This can be removed for applications
# that need different build settings.
apply_standard_settings(${BINARY_NAME})

# Add preprocessor definitions for the application ID.
add_definitions(-DAPPLICATION_ID="${APPLICATION_ID}")

# Add dependency libraries. Add any application-specific dependencies here.
target_link_libraries(${BINARY_NAME} PRIVATE flutter)
target_link_libraries(${BINARY_NAME} PRIVATE PkgConfig::GTK)

target_include_directories(${BINARY_NAME} PRIVATE "${CMAKE_SOURCE_DIR}")

file(GLOB_RECURSE LINUX_BROKER_RUST_SOURCES CONFIGURE_DEPENDS
  "${REPOSITORY_ROOT}/native/rust_helper/src/*.rs")
add_custom_command(
  OUTPUT "${LINUX_BROKER_HELPER}"
  COMMAND ${CMAKE_COMMAND} -E make_directory
    "${CMAKE_BINARY_DIR}/intermediates_do_not_run/libexec"
  COMMAND ${CMAKE_COMMAND} -E env
    "CARGO_TARGET_DIR=${CMAKE_BINARY_DIR}/linux_broker_cargo_target"
    "${CARGO_EXECUTABLE}" build --locked --release
    --manifest-path "${REPOSITORY_ROOT}/native/rust_helper/Cargo.toml"
    --bin gui_shell_linux_broker --features linux-desktop-broker
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${CMAKE_BINARY_DIR}/linux_broker_cargo_target/release/gui_shell_linux_broker"
    "${LINUX_BROKER_HELPER}"
  DEPENDS
    "${REPOSITORY_ROOT}/native/rust_helper/Cargo.toml"
    "${REPOSITORY_ROOT}/native/rust_helper/Cargo.lock"
    ${LINUX_BROKER_RUST_SOURCES}
  WORKING_DIRECTORY "${REPOSITORY_ROOT}"
  VERBATIM
)
add_custom_target(gui_shell_linux_broker DEPENDS "${LINUX_BROKER_HELPER}")
add_dependencies(${BINARY_NAME} gui_shell_linux_broker)
