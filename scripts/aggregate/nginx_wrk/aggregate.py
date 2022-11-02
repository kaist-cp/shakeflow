# Reads csv files in `scripts/csvs/nginx_wrk`, and aggregates then into csv files
# that are used in `plot.py`.

import copy
import csv
import pathlib
import numpy as np
import scipy.stats
import os
import re

COR = "$\\mathbf{C_{Orig}}$"
SF = "$\\mathbf{C_{SF}}$"
MEL = "$\\mathbf{M}$"

for di in ['rx', 'tx']:
# for di in ['tx']:
    current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
    corlist = list(pathlib.Path(f'{current_dir}/../../csvs/nginx_wrk/{di}').glob('*corig*.csv'))
    sflist = list(pathlib.Path(f'{current_dir}/../../csvs/nginx_wrk/{di}').glob('*csf*.csv'))
    mellist = list(pathlib.Path(f'{current_dir}/../../csvs/nginx_wrk/rx').glob('*m*.csv'))
    corlist.extend(sflist)
    corlist.extend(mellist)
    pathlist = corlist

    cor = [[] for _ in range(16)]
    sf = [[] for _ in range(16)]
    mel = [[] for _ in range(16)]
    for n in pathlist:
        with open(n, 'r') as f:
            n = str(n)
            if 'corig' in n:
                target = 'cor'
            elif 'csf' in n:
                target = 'sf'
            elif 'm' in n:
                target = 'mel'
            else:
                continue

            csvreader = csv.reader(f)
            lines = list(filter(lambda s: re.search('Transfer/sec', s), f.read().splitlines()))
            assert(len(lines) == 16)

            # `bw_avg` is in Gbps
            for (idx, line) in enumerate(lines):
                numbers = re.findall(r"[-+]?\d*\.\d+|\d+", line)
                assert len(numbers) == 1
                bw_avg = float(numbers[0])
                if 'MB' in line:
                    bw_avg = bw_avg / 1024 * 8
                elif 'GB' in line:
                    bw_avg = bw_avg * 8
                else:
                    print(n)
                    assert False

                if target == 'cor':
                    cor[idx].append(bw_avg)
                elif target == 'sf':
                    sf[idx].append(bw_avg)
                elif target == 'mel':
                    mel[idx].append(bw_avg)
                else:
                    assert False

    std = []

        # print(cor[14])
        # print(sf[14])
    with open(pathlib.Path(f'{current_dir}/1core_{di}.csv'), 'w') as f:
        f.write(f'size\ttype\tthroughput\n')
        for i in range(8):
            for b in cor[i]:
                f.write(f'{4*(1 << (i*2))}\t{COR}\t{b}\n')
            for b in sf[i]:
                f.write(f'{4*(1 << (i*2))}\t{SF}\t{b}\n')
            for b in mel[i]:
                f.write(f'{4*(1 << (i*2))}\t{MEL}\t{b}\n')
    with open(pathlib.Path(f'{current_dir}/8core_{di}.csv'), 'w') as f:
        f.write(f'size\ttype\tthroughput\n')
        for i in range(8,16):
            for b in cor[i]:
                f.write(f'{4 * (1 << (2*(i-8)))}\t{COR}\t{b}\n')
            for b in sf[i]:
                f.write(f'{4 * (1 << (2*(i-8)))}\t{SF}\t{b}\n')
            for b in mel[i]:
                f.write(f'{4 * (1 << (2*(i-8)))}\t{MEL}\t{b}\n')

    # l1 = []
    # l2 = []
    # l3 = []
    # l4 = []
    # for i in range(10):
    #     c = cor[i]
    #     s = sf[i]
    #     m = mel[i]
    #     x = np.average(s)/ np.average(c) - 1
    #     l1.append(x)
    #     l2.append((np.average(s)- np.average(c)) / np.std(c))
    #     l3.append((np.average(s)- np.average(c)) / np.std(s))
    #     l4.append((np.average(m)/ np.average(s)) - 1)
