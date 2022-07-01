#!/bin/sh
set -eu

get_ip() {
    CURR_IP="$(dig -4 +short myip.opendns.com @resolver1.opendns.com)"
}

get_ip
INIT_IP="$CURR_IP"

while true
do
    sleep 10
    get_ip

    if [ "$CURR_IP" != "$INIT_IP" ]
    then
        MSG="Public IP Changed! Old IP $INIT_IP new IP $CURR_IP"
        echo "$MSG"
        aws sns publish --target-arn "arn:aws:sns:$AWS_DEFAULT_REGION:$AWS_ACCOUNT_ID:conesnake" --message "$MSG"
        INIT_IP="$CURR_IP"
    fi
done
