# CMake file to get and integrate the sysinfo library into a C/C++ project
# Requires cargo and rust installed on the system

cmake_minimum_required(VERSION 3.21 FATAL_ERROR)

project(sysinfo
  VERSION 0.38.3
  DESCRIPTION "Cross-platform library to fetch system information"
  LANGUAGES C
)

# Project options
option(BUILD_SHARED_LIBS OFF "Build static or shared library")
option(SYSINFO_BUILD_INSTALL OFF "Build install target")
set(TARGET_NAME ${PROJECT_NAME} CACHE STRING "Name for the library and binary files")
set(SYSINFO_BUILD_TYPE "Release" CACHE STRING "On MSVC, which build type to use")   # Used only on MSVC

# Ensure cargo is installed
set(CARGO_EXECUTABLE)
find_program(CARGO_EXECUTABLE NAMES cargo)
if("${CARGO_EXECUTABLE}" STREQUAL "CARGO_EXECUTABLE-NOTFOUND")
  message(FATAL_ERROR "Cargo was not found on the system. Check https://rust-lang.org/learn/get-started")
endif()

# Copy the sysinfo.h header into the ${CMAKE_CURRENT_BINARY_DIR}/include/sysinfo folder to allow for a more readable header
# inclusion in C/C++ code
set(SYSINFO_INCLUDE_DIR "${CMAKE_CURRENT_BINARY_DIR}/include")
set(SYSINFO_HEADER_FILE_SOURCE "${CMAKE_CURRENT_SOURCE_DIR}/src/sysinfo.h")
set(SYSINFO_HEADER_FILE_DESTINATION "${SYSINFO_INCLUDE_DIR}/sysinfo/sysinfo.h")
if(NOT EXISTS ${SYSINFO_HEADER_FILE_DESTINATION})
  file(MAKE_DIRECTORY "${SYSINFO_INCLUDE_DIR}/${PROJECT_NAME}")
endif()

file(COPY_FILE 
  ${SYSINFO_HEADER_FILE_SOURCE} ${SYSINFO_HEADER_FILE_DESTINATION}
  RESULT SYSINFO_HEADER_COPY_RESULT
  ONLY_IF_DIFFERENT
)

if(NOT SYSINFO_HEADER_COPY_RESULT STREQUAL "0")
  message(FATAL_ERROR "Unable to copy header 'sysinfo.h' from '${SYSINFO_HEADER_FILE_SOURCE}' to '${SYSINFO_HEADER_FILE_DESTINATION}': ${SYSINFO_HEADER_COPY_RESULT}")
endif()

if(WIN32)
  set(SYSINFO_SHARED_LIBRARY_EXTENSION "dll")
  set(SYSINFO_STATIC_LIBRARY_EXTENSION "lib")
  set(SYSINFO_LIBRARY_PREFIX "")
else()
  set(SYSINFO_SHARED_LIBRARY_EXTENSION "so")
  set(SYSINFO_STATIC_LIBRARY_EXTENSION "a")
  set(SYSINFO_LIBRARY_PREFIX "lib")
endif()

if(NOT MSVC)
  set(SYSINFO_BUILD_TYPE ${CMAKE_BUILD_TYPE})
  string(TOLOWER ${SYSINFO_BUILD_TYPE} SYSINFO_BUILD_TYPE)
endif()

set(SYSINFO_CMAKE_LIBRARY_TYPE "STATIC")
if(BUILD_SHARED_LIBS)
  set(SYSINFO_CMAKE_LIBRARY_TYPE "SHARED")
endif()

# Add a custom command to build the library and a custom target that depends on the output of the command
set(SYSINFO_LIBRARY_PATH "${CMAKE_CURRENT_BINARY_DIR}/${SYSINFO_BUILD_TYPE}")
set(SYSINFO_LIBRARY_NAME ${SYSINFO_LIBRARY_PREFIX}${TARGET_NAME})

if(BUILD_SHARED_LIBS)
  set(SYSINFO_LIBRARY_OUTPUT ${SYSINFO_LIBRARY_PATH}/${SYSINFO_LIBRARY_NAME}.${SYSINFO_SHARED_LIBRARY_EXTENSION})
else()
  set(SYSINFO_LIBRARY_OUTPUT ${SYSINFO_LIBRARY_PATH}/${SYSINFO_LIBRARY_NAME}.${SYSINFO_STATIC_LIBRARY_EXTENSION})
endif()

add_custom_command(
  COMMAND ${CARGO_EXECUTABLE}
  ARGS "rustc" "--manifest-path" "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
               "--features=c-interface" "$<$<CONFIG:RELEASE>:--release>"
               "--crate-type" $<IF:$<BOOL:${BUILD_SHARED_LIBS}>,"cdylib","staticlib">
               "--target-dir" "${CMAKE_CURRENT_BINARY_DIR}"
  DEPENDS ${SYSINFO_HEADER_FILE_DESTINATION} # CMake will re-run the command whenever the file changes
  OUTPUT ${SYSINFO_LIBRARY_OUTPUT}
)
add_custom_target(${TARGET_NAME}_rust ALL DEPENDS ${SYSINFO_LIBRARY_OUTPUT})

# Generate the imported library for sysinfo and link it to the custom target to ensure the custom command to be called
add_library(${TARGET_NAME} ${SYSINFO_CMAKE_LIBRARY_TYPE} IMPORTED GLOBAL)
add_library(${TARGET_NAME}::${TARGET_NAME} ALIAS ${TARGET_NAME})

# Set Imported Library properties
if(BUILD_SHARED_LIBS)
  set(SYSINFO_LIBRARY_IMPORTED_LOCATION
      ${SYSINFO_LIBRARY_PATH}/${SYSINFO_LIBRARY_NAME}.${SYSINFO_SHARED_LIBRARY_EXTENSION}
  )
else()
  set(SYSINFO_LIBRARY_IMPORTED_LOCATION
      ${SYSINFO_LIBRARY_PATH}/${SYSINFO_LIBRARY_NAME}.${SYSINFO_STATIC_LIBRARY_EXTENSION}
  )
endif()

set_target_properties(
  ${TARGET_NAME}
  PROPERTIES IMPORTED_LOCATION ${SYSINFO_LIBRARY_IMPORTED_LOCATION}
             IMPORTED_IMPLIB ${SYSINFO_LIBRARY_PATH}/${SYSINFO_LIBRARY_NAME}.${SYSINFO_STATIC_LIBRARY_EXTENSION}
)
target_include_directories(${TARGET_NAME} INTERFACE ${SYSINFO_INCLUDE_DIR})

if(SYSINFO_BUILD_INSTALL)
  get_filename_component(file ${SYSINFO_HEADER_FILE_DESTINATION} ABSOLUTE)
  get_filename_component(dir ${SYSINFO_HEADER_FILE_DESTINATION} DIRECTORY)
  string(REPLACE "${SYSINFO_INCLUDE_DIR}/sysinfo" "" dir ${dir})
  set(dir "${CMAKE_INSTALL_INCLUDEDIR}/${TARGET_NAME}/${dir}")
  if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.31")
    # Policy CMP0177
    cmake_path(SET dir NORMALIZE "${dir}")
  endif()
  install(FILES ${file} DESTINATION ${dir})

  # Configuration
  configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/${TARGET_NAME}Config.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_NAME}Config.cmake
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${TARGET_NAME}
  )
  write_basic_package_version_file(
    ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_NAME}ConfigVersion.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
  )
  # Package
  install(FILES ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_NAME}Config.cmake
                ${CMAKE_CURRENT_BINARY_DIR}/${TARGET_NAME}ConfigVersion.cmake
          DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${TARGET_NAME}
  )
  install(
    EXPORT ${TARGET_NAME}Targets
    FILE ${TARGET_NAME}Targets.cmake
    NAMESPACE ${TARGET_NAME}::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${TARGET_NAME}
  )
  install(
    TARGETS ${TARGET_NAME}
    EXPORT ${TARGET_NAME}Targets
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  )

  #
  # CPack
  #
  set(CPACK_GENERATOR DEB RPM TXZ TGZ)
  set(CPACK_PACKAGE_VERSION ${PROJECT_VERSION})
  set(CPACK_PACKAGE_NAME ${TARGET_NAME})
  set(CPACK_PACKAGE_DESCRIPTION_SUMMARY ${PROJECT_DESCRIPTION})
  set(CPACK_PACKAGE_FILE_NAME
      "${CPACK_PACKAGE_NAME}-${CPACK_PACKAGE_VERSION}.${CMAKE_SYSTEM_PROCESSOR}"
  )
  set(CPACK_PACKAGE_RPM_DIR "${CMAKE_CURRENT_BINARY_DIR}/_CPack_Packages/Linux/RPM")
  set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Guillaume Gomez <guillaume1.gomez@gmail.com>")
  include(CPack)
endif()