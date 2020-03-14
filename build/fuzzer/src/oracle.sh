#!/bin/bash

fuzzerResponse=fuzzerResponse
oracleResponse=oracleResponse

mkfifo $fuzzerResponse
mkfifo $oracleResponse

rm ./logs/oracle_out.txt
#./oracle $1  &> output | $output > ./logs/oracle_out.txt
./oracle $1 &>> ./logs/oracle_out.txt
