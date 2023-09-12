# run.sh can use --with-cuda or the model param as 7b for smaller machines
# it will take a minute to "come online" clean run it's downloading a few GB of model


$(cd kinbot/matrix_bot && cargo run &)

wait