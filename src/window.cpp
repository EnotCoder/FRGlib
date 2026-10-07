#include "frglib/window.hpp"

namespace frglib {

    window::~window() {
        close();
    }

    bool window::create(const char* title, int width, int height, bool resize) {
        if (window_) {
            SDL_SetError("Window is already created");
            return false;
        }

        SDL_WindowFlags flags = 0;

        if (resize){
            // = заменяет значение, а |= включает нужный флаг, сохраняя остальные
            flags |= SDL_WINDOW_RESIZABLE;
        }

        window_ = SDL_CreateWindow(
            title, width, height, SDL_WINDOW_RESIZABLE
        );

        return window_ != nullptr;
    }

    void window::close() {
        if (window_) {
            SDL_DestroyWindow(window_);
            window_ = nullptr;
        }
    }

}