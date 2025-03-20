#!/bin/bash

if ! command -v cargo &> /dev/null
then
   direnv reload
fi

cargo fmt
