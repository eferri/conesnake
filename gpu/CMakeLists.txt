project(gen_board LANGUAGES CUDA CXX)

enable_language(CUDA)
cmake_minimum_required(VERSION 3.22)

set(JSON_BuildTests OFF CACHE INTERNAL "")
add_subdirectory(submodules/json)

add_executable(gen_board
    gen_board.cu
)

target_compile_features(gen_board PUBLIC cuda_std_17)
set_target_properties(gen_board PROPERTIES
    CUDA_ARCHITECTURES "75;86"
)

target_link_libraries(gen_board PRIVATE nlohmann_json::nlohmann_json)
