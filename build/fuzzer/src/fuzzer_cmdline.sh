#!/bin/bash

o_err=0
dut_err=0
temp=0
mm_cnt=0

fuzz_start_time=`date +s%.%N`
echo "Staring fuzzing at ${fuzz_start_time}"
echo "Starting fuzzing" > log/${fuzz_start_time}.log
echo "" > log/${fuzz_start_time}.txt

while true; do
  SEED=$(find ./seeds -type f | shuf -n 1)
  radamsa $SEED > input.txt

  LastLine=$( tail -n 1 input.txt )
  #echo "$LastLine"

  #prevents dut from getting stuck in infinate loop
  if [[ "$LastLine" != "***" ]]
  then
    echo "***" >> input.txt
  fi

  cat input.txt >> log/${fuzz_start_time}.txt
  

  o_resp=$(netcat -w 30 127.0.0.1 $1 <input.txt 2>o_err)
  dut_resp=$(netcat -w 30 0.0.0.0 $2 <input.txt 2>dut_err) #replace this with dut when it works

  #clean up json
  o_resp_c=$(jq -rScM . <<< "$o_resp")
  dut_resp_c=$(jq -rScM . <<< "$dut_resp")

  #equivalence checking
  mm_time=`date +s%.%N`
  if [ -z "$o_resp" ] #check is string is empty
  then
    echo "Oracle not responding"
    sleep 30
  elif [ -z "$dut_resp" ] #check is string is empty
  then
    echo "DUT not responding"
    sleep 30
  elif [ "$o_resp_c" != "$dut_resp_c" ] 
  then
    ((mm_cnt=mm_cnt+1))
    cp input.txt ./mismatches/mm-${fuzz_start_time}--${mm_cnt}--${mm_time}.txt
    STR="${mm_cnt}: ${mm_time} *** Mismatch: 
    $o_resp_c
    !=
    $dut_resp_c"
    echo "$STR" >> log/${fuzz_start_time}.log
  elif [ "$o_resp_c" == "$dut_resp_c" ] 
  then
    #STR="Match: 
    #$o_resp_c 
    #== 
    #$dut_resp_c"
    #echo "$STR"
    temp=1
  else
    echo "Error"
  fi

  #crash handling
  if [ $o_err -gt 127 ]; then
    cp input.txt ./crashes/ocrash-${fuzz_start_time}-${mm_time}.txt
    echo "Crash found!"
  elif [ $dut_err -gt 127 ]; then
    cp input.txt ./crashes/dcrash-${fuzz_start_time}-${mm_time}.txt
    echo "Crash found!"
  fi


done



