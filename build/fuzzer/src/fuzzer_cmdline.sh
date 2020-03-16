#!/bin/bash

oracle_pipe=/tmp/oraclepipe
trap "rm -f $oracle_pipe" EXIT

if [[ ! -p $oracle_pipe ]]; then
    mkfifo $oracle_pipe
fi

#start oracle
#screen -dmS oracle sh -c "./src/oracle.sh $1; exec bash"

while true; do
  SEED=$(find ./seeds -type f | shuf -n 1)
  radamsa $SEED > input.txt

  o_resp=$(netcat 127.0.0.1 $1 <input.txt )
  dut_resp=$(netcat 127.0.0.1 $1 <input.txt ) #replace this with dut when it works
  
#  if read -t 5 oracle_line <$oracle_pipe; then
#    echo "Here 56"
##    if [[ "$oracle_line" == 'quit' ]]; then
 #     echo "Here 57"
 #     break
 #   fi
 #   echo $oracle_line
 # fi

  #equivalence checking
  if [ -z "$o_resp" ] || [ -z "$dut_resp" ] #check is string is empty
  then
    echo "Oracle not running"
    sleep 5
  elif [ "$o_resp" != "$dut_resp" ] 
  then
    echo "Mismatch: $o_resp == $dut_resp"
#   echo "Oracle: $oracle_line"
  elif [ "$o_resp" == "$dut_resp" ] 
  then
    echo "Match: $o_resp == $dut_resp"
  else
    echo "Error"
  fi

  #crash handling
  if [ $? -gt 127 ]; then
    cp input.txt ./crashes/crash-`date +s%.%N`.txt
    echo "Crash found!"
  fi
done



