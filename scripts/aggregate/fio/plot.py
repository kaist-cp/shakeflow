# Takes the csv files outputed by `aggregate.py`,
# and creates plot svg and png files.

import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
import pandas as pd
import pathlib
import os

COR = "$\\mathbf{C_{Orig}}$"
SF = "$\\mathbf{C_{SF}}$"
MEL = "$\\mathbf{M}$"


plots = [COR, SF, MEL]
point_color = {
    COR : '#dc5f57',
    SF : '#57dc5f',
    MEL : '#5f57dc',
}
hatches = ["//",  "-", "xx",]

def change_width(ax, new_value) :
    for i, patch in enumerate(ax.patches):
        current_width = patch.get_width()
        diff = current_width - new_value

        # we change the bar width
        patch.set_width(new_value)

        # we recenter the bar
        if i // 13 == 0:
            offset = diff
        elif i // 13 == 1:
            offset = diff * 0.5
        elif i // 13 == 2:
            offset = 0
        patch.set_x(patch.get_x() + offset)
    for i, eb in enumerate(ax.lines):
        diff = current_width - new_value
        xy = eb.get_xydata()

        # we recenter the bar
        if i < 39:
            offset = diff
        elif i < 78:
            offset = diff * 0.5
        else:
            offset = 0
        eb.set_xdata([xy[0][0] + offset - diff*0.5, xy[1][0] + offset - diff*0.5])

current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
sns.set()
sns.set_theme(style="ticks", font_scale=2, rc={'figure.figsize':(10,5)})

for di in ['rx', 'tx']:
    for measure in ['throughput']:
    # for measure in ['throughput', 'IOPS']:
        for blocksize in ['4k', '256k']:
            filename = f'{measure}_{blocksize}_{di}'
            dt = pd.read_csv(f'{current_dir}/{filename}.csv', sep='\t')
            dt['type'] = pd.Categorical(dt['type'], categories=plots, ordered=True)

            fig = plt.figure()
            ax = plt.axes()

            # bar = sns.barplot(x="IOdepth", y=measure, hue="type", data=dt, palette=point_color)
            bar = sns.barplot(x="IOdepth", y=measure, hue="type", data=dt, palette=point_color, ci='sd', errwidth=1.4, capsize=0.07)

            # Loop over the bars
            for bars, hatch in zip(ax.containers, hatches):
                # Set a different hatch for each group of bars
                for b in bars:
                    b.set_hatch(hatch)
            # create the legend again to show the new hatching
            ax.legend()
            # plt.show()
            bar.set_xlabel('I/O depth')
            if measure == 'throughput':
                bar.set_ylabel('Throughput [Gbps]')
            else:
                bar.set_ylabel('IOPS')

            # remove every other xtick
            xlabels = ax.get_xticklabels()
            for i, l in enumerate(xlabels):
                if i%2 == 1:
                    xlabels[i] = None
            ax.set_xticklabels(xlabels)
            xs = list(filter(lambda x: x%2==0, ax.get_xticks()))
            ax.set_xticks(xs)
            change_width(ax, .2)

            if measure == 'IOPS':
                ylabels = [f'{int(y)}{"k" if y else ""}' for y in ax.get_yticks()/1000]
                ax.set_yticklabels(ylabels)

            plt.legend(loc='lower right')
            plt.tight_layout()
            output_name = f'figure-fio-{blocksize}-{"server" if di == "tx" else "client"}'
            # plt.savefig(f'{current_dir}/{output_name}.png')
            plt.savefig(f'{current_dir}/{output_name}.svg')
