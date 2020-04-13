#!/bin/bash
for filename in ./seeds/*.txt; do
  echo "$filename"

  o_resp=$(netcat -w 30 127.0.0.1 $1 <$filename )
  dut_resp=$(netcat -w 30 0.0.0.0 $2 <$filename ) #replace this with dut when it works

  #clean up json
  o_resp_c=$(jq -rScM . <<< "$o_resp")
  dut_resp_c=$(jq -rScM . <<< "$dut_resp")

  #equivalence checking
  mm_time=`date +s%.%N`
  if [ -z "$o_resp" ]  #check is string is empty
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
    STR="${mm_cnt}: ${mm_time} *** Mismatch: 
    $o_resp_c
    !=
    $dut_resp_c"
    echo "$STR" 
  elif [ "$o_resp_c" == "$dut_resp_c" ] 
  then
    echo "Passed!"
  else
    echo "Error"
  fi

done