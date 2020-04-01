# BiBiFi

Even more "roll your own".

## Status

Please see the [Work Distribution](https://github.tamu.edu/csce-489-713-lads/BiBiFi/wiki/Work-Distribution) page in the
README.

## FUZZER

1. Install radamsa fuzzer (https://gitlab.com/akihe/radamsa). If you installed the files fron Dr. Ritchey then you should already have the radmasa files. Just add the path for radmasa to your .bashrc file. To test run ```echo "aaa" | radamsa```. 
2.  In the BiBiFI/build/fuzzer directory, you should have the following directories/files. If you don't have them please add them.
	- src: fuzzer scripts.
	- seeds: Seed files for fuzzer. Randomly choosen.
	- mismatches: The inputs that caused the mismatch are stored here. The format is "mm\-\<timestamp of fuzzer run>--\<mismatch number>--\<timestamp when mismatch occured>.txt".
	- log: Timestamped logs will be stored here. When you run the fuzzer the timestamp will be printed so you can find the log. The ".log" files contain the mismatches. The ".txt" contains all the inputs run by the fuzzer.
	- crashes: If one of the programs crashes the input file that caused it would be stored here.
	- oracle executable
3. To run the fuzzer you need three terminal tabs:
	- Tab 1: Run the oracle from the BiBiFI/build/fuzzer directory using ``./src/oracle.sh <4 digit oracle port number>``
	- Tab 2: Run the fuzzer from the BiBiFI/build/fuzzer directory using ``./src/fuzzer_cmdline.sh <oracle port number> <dut port number>``
	- Tab 3: Run the dut from  the BiBiFI/build/fuzzer directory using `` ./code_mint.sh <dut prot number> ``
	-- Make sure the other teams files are in the same directory that BiBiFI is in
	--You can replace code_mint with the other teams bash files.
	--You can also just go into their directories and run ``./server <port number>`` but the fuzzer wont be able to record their crashes 
	
