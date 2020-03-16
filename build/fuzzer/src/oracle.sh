#!/bin/bash

pipe=/tmp/oraclepipe

if [[ ! -p $pipe ]]; then
    echo "Reader not running"
    exit 1
fi


#rm ./logs/oracle_out.txt
#./oracle $1  &> output | $output > ./logs/oracle_out.txt
#./oracle $1 &>> ./logs/oracle_out.txt
./oracle $1 &> $pipe
echo $pipe
