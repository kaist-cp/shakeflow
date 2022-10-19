#Config collected info as well

[ -z "$FSIZE" ] && echo "FSIZE=$FSIZE" && exit -1

TBASE=$TBASE/
[ ! -e "$TBASE" ] && echo "base directory is not at $TBASE" && exit -1
[ -z "$OUT_FILE" ] && OUT_FILE=/tmp/
#rm -rf $OUT_FILE/*

Test=$1
[ -z "$Test" ] && echo "$0 ERROR: not test defined" && exit -1;
[  ! -e "$Test/test.sh" ] && echo "No File" && exit -1

echo NOCONFIG=$NOCONFIG
if [ -z "$NOCONFIG" ]; then
	echo "source $Test/config.sh"
	source $Test/config.sh >> $OUT_FILE/test_raw.txt
else
	echo "[+] Skipping config!"
fi

[ -z "$repeat" ] && repeat=1
[ -z "$DELAY" ] && DELAY=50

export TIME=70

rm -rf $OUT_FILE/result.txt

#echo "[+] fetching all inodes for $TBASE/nvme/mount/nginx.$FSIZE"
#time ls $TBASE/nvme/mount/nginx.$FSIZE > /dev/null # fetch all inodes

echo "$date starting ($Test $repeat [$DELAY])"
for i in `seq 1 $repeat`; do
	date=`date +"%H:%M.%s:"`
	export OUT_FILE=$OUT_FILE
	export OUTPUT=$OUT_FILE
	echo $OUT_FILE | sudo tee /dev/kmsg
	$Test/test.sh >> $OUT_FILE/test_raw.txt &
	testid=$!
	echo "$date $Test/test.sh & $OUT_FILE"
	sleep $DELAY
	# sudo -E $TBASE/DataCollector/collect_membw.sh &>> $OUT_FILE/result.txt
	echo "collect done"
	# collection is ±20sec
	echo "$date waiting for test and collector ($Test)"
	if [ -e $Test/wait.sh ]; then
                echo "using $Test/wait.sh"
                $Test/wait.sh
        else
		echo "using sleep $[$TIME-$DELAY]"
		sleep $[$TIME-$DELAY]
		sleep 5 # write output
        fi
	#echo "$date running post ($Test)"
	#DataCollector/post_process.sh &>> $OUT_FILE/post.txt
done
date=`date +"%H:%M.%s:"`
echo "$date Done ($Test)"
