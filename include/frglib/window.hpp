#pragma once

#include <SDL3/SDL.h>

namespace frglib{

    class window{
        public:
            window() = default;
            ~window();

            // запрет на клонирование
            window(const window&) = delete;
            // запрет на присваение
            window& operator = (const window&) = delete;  

            bool create(const char* title, int width, int height, bool resize = true);
            void close();

        private:
            SDL_Window* window_ = nullptr;
    };

}