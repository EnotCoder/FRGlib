cmake_minimum_required(VERSION 3.20)
project(frglib LANGUAGES CXX)

find_package(SDL3 CONFIG REQUIRED)

add_library(frglib SHARED
    src/window.cpp
    src/python_api.cpp
)

target_compile_features(frglib PRIVATE cxx_std_17)

target_include_directories(frglib PUBLIC
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
)

target_link_libraries(frglib PUBLIC SDL3::SDL3)