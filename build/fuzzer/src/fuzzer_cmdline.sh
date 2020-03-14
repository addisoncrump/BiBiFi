#!/bin/bash

fuzzerResponse=fuzzerResponse
oracleResponse=oracleResponse
rm ./logs/fuzzer_out.txt


while true; do
	SEED=$(find ./seeds -type f | shuf -n 1)
  radamsa $SEED > input.txt
  netcat 127.0.0.1 $1 <input.txt &>>./logs/fuzzer_out.txt  2>&1
  #netcat 127.0.0.1 $1 <input.txt &>$fuzzerResponse  2>&1
  if [ $? -gt 127 ]; then
    cp input.txt ./crashes/crash-`date +s%.%N`.txt
    echo "Crash found!"
  fi
done