# Reads csv files in `scripts/csvs/fio`, and aggregates then into csv files
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
    current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
    corlist = list(pathlib.Path(f'{current_dir}/../../csvs/fio/{di}').glob('*corig*.csv'))
    sflist = list(pathlib.Path(f'{current_dir}/../../csvs/fio/{di}').glob('*csf*.csv'))
    mellist = list(pathlib.Path(f'{current_dir}/../../csvs/fio/rx').glob('*m*.csv'))
    corlist.extend(sflist)
    corlist.extend(mellist)
    pathlist = corlist

    cor = [[] for _ in range(26)]
    sf = [[] for _ in range(26)]
    mel = [[] for _ in range(26)]

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
            lines = f.read().splitlines()
            if len(lines) != 104:
                print(n)
                assert len(lines) == 104

            # `bw_avg` is in Gbps
            for (idx, i) in enumerate(range(2, len(lines), 4)):
                numbers = re.findall(r"[-+]?\d*\.\d+|\d+", lines[i])
                assert len(numbers) == 6
                bw_avg = float(numbers[3])
                if 'KiB/s' in lines[i]:
                    bw_avg = bw_avg / 1024 / 1024 * 8
                elif 'MiB/s' in lines[i]:
                    bw_avg = bw_avg / 1024 * 8
                else:
                    assert False
                numbers = re.findall(r"[-+]?\d*\.\d+|\d+", lines[i+1])
                assert len(numbers) == 5
                iops_avg = float(numbers[2])

                if target == 'cor':
                    cor[idx].append((bw_avg, iops_avg))
                elif target == 'sf':
                    sf[idx].append((bw_avg, iops_avg))
                elif target == 'mel':
                    mel[idx].append((bw_avg, iops_avg))
                else:
                    assert False

    std = []

    # for expr in [range(2)]: # throughput, iops
    for expr in [0]: # throughput, iops
        expr_title = ['throughput', 'IOPS'][expr]
        with open(pathlib.Path(f'{current_dir}/{expr_title}_4k_{di}.csv'), 'w') as f:
            f.write(f'IOdepth\ttype\t{expr_title}\n')
            for i in range(13):
                for b in cor[i]:
                    f.write(f'{1 << i}\t{COR}\t{b[expr]}\n')
                for b in sf[i]:
                    f.write(f'{1 << i}\t{SF}\t{b[expr]}\n')
                for b in mel[i]:
                    f.write(f'{1 << i}\t{MEL}\t{b[expr]}\n')
        with open(pathlib.Path(f'{current_dir}/{expr_title}_256k_{di}.csv'), 'w') as f:
            f.write(f'IOdepth\ttype\t{expr_title}\n')
            for i in range(13,26):
                for b in cor[i]:
                    f.write(f'{1 << (i-13)}\t{COR}\t{b[expr]}\n')
                for b in sf[i]:
                    f.write(f'{1 << (i-13)}\t{SF}\t{b[expr]}\n')
                for b in mel[i]:
                    f.write(f'{1 << (i-13)}\t{MEL}\t{b[expr]}\n')
    # for expr in range(2): # throughput, iops
    for expr in [0]: # throughput, iops
        l1 = []
        l2 = []
        l3 = []
        l4 = []
        for i in range(13):
            c = list(map(lambda x: x[expr], cor[i]))
            s = list(map(lambda x: x[expr], sf[i]))
            m = list(map(lambda x: x[expr], mel[i]))
            x = np.average(s)/ np.average(c) - 1
            l1.append(x)
            l2.append((np.average(s)- np.average(c)) / np.std(c))
            l3.append((np.average(s)- np.average(c)) / np.std(s))
            l4.append((np.average(s)/ np.average(m)) - 1)

        l1 = []
        l2 = []
        l3 = []
        l4 = []
        for i in range(13,26):
            c = list(map(lambda x: x[expr], cor[i]))
            s = list(map(lambda x: x[expr], sf[i]))
            m = list(map(lambda x: x[expr], mel[i]))
            x = np.average(s)/ np.average(c) - 1
            l1.append(x)
            l2.append((np.average(s)- np.average(c)) / np.std(c))
            l3.append((np.average(s)- np.average(c)) / np.std(s))
            l4.append((np.average(s)/ np.average(m)) - 1)
