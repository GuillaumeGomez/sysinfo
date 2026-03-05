cmake_minimum_required(VERSION 3.21 FATAL_ERROR)

project(
    sysinfo_c_example
    VERSION 0.38.3
    DESCRIPTION "Example binary of sysinfo C interface"
    LANGUAGES C
)

add_executable(simple "${CMAKE_CURRENT_SOURCE_DIR}/simple.c")

# Link the sysinfo alias target available from the parent
target_link_libraries(simple PRIVATE sysinfo::sysinfo)

# Enable pedantic building to ensure code quality
target_compile_options(simple PRIVATE
  $<$<C_COMPILER_ID:MSVC>:/W4 /WX>
  $<$<NOT:$<C_COMPILER_ID:MSVC>>:-Wall -Wextra -Wpedantic -Werror>
)
