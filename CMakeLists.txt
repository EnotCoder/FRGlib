cmake_minimum_required(VERSION 3.20)
project(frglib LANGUAGES CXX)

find_package(SDL3 CONFIG REQUIRED)

add_executable(frglib src/main.cpp)
target_compile_features(frglib PRIVATE cxx_std_17)
target_link_libraries(frglib PRIVATE SDL3::SDL3)