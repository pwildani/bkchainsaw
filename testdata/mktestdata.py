#!/usr/bin/python3
import random
import sys
count = int(sys.argv[1])
for _ in range(count):
    print(random.randint(0,2**64-1))

