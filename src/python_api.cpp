#include "frglib/window.hpp"
#include <SDL3/SDL.h>

static frglib::window* win = nullptr;

extern "C" {

    int frg_create(const char* title, int width, int height, int resize) {
        if (win) {
            return 0;
        }

        if (!SDL_InitSubSystem(SDL_INIT_VIDEO)) {
            return 0;
        }

        // создание объекта окно
        win = new frglib::window;

        if (!win->create(title, width, height, resize != 0)) {
            delete win;
            win = nullptr;
            SDL_QuitSubSystem(SDL_INIT_VIDEO);
            return 0;
        }

        return 1;
    }

    int frg_poll() {
        if (!win) {
            return 0;
        }

        SDL_Event event;

        while (SDL_PollEvent(&event)) {
            if (event.type == SDL_EVENT_QUIT ||
                event.type == SDL_EVENT_WINDOW_CLOSE_REQUESTED) {
                return 0;
            }
        }

        return 1;
    }

    void frg_close() {
        if (win) {
            delete win;
            win = nullptr;
            SDL_QuitSubSystem(SDL_INIT_VIDEO);
        }
    }

}