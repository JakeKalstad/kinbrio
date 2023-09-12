# run.sh can use --with-cuda or the model param as 7b for smaller machines
# it will take a minute to "come online" clean run it's downloading a few GB of model
#!/bin/bash
cd kinbot/kinbot-gpt && sudo ./run.sh --model 13b &

wait