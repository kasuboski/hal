#!/bin/sh
find . -name "*.rs" -type f -not -path "./target/*" -exec rustfmt {} \;
