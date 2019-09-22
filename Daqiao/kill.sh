#!/bin/bash

ps aux| grep "debug/daqiao" | grep -v grep | awk '{print $2}' | xargs kill -9
