#!/bin/bash

#pipe=/tmp/oraclepipe

#if [[ ! -p $pipe ]]; then
#    echo "Reader not running"
#    exit 1
#fi

#check if tcp port is in use
port_open=$(ss -tulpn | grep -c $1)
if [ $port_open -eq 1 ]; then
  echo "Port already in use"
  exit
fi

#./oracle $1 &> $pipe
while true; do
	./oracle $1 
	cp input.txt ./crashes/o_crash-`date +s%.%N`.txt
  echo "Crash found!"
  sleep 5
done