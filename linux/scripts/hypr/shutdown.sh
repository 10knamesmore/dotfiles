#!/usr/bin/bash

kill -9 $(pidof waybar)
kill -9 $(pidof hypridle)

"$scripts_dir"/hypr/hyprshutdown

