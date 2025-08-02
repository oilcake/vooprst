path_to_video := env_var_or_default('PATH_TO_VIDEO', 'samples/shiny_hand_01.mov')
debug := 'target/debug/app'

run:
    cargo build
    {{ debug }} {{ path_to_video }}
