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

current_dir = pathlib.Path(f'{os.path.dirname(os.path.abspath(__file__))}')
sns.set()
sns.set_theme(style="ticks", font_scale=2, rc={'figure.figsize':(10,5)})

for di in ['rx', 'tx']:
    for core in [1, 8]:
        filename = f'{core}core_{di}'
        dt = pd.read_csv(f'{current_dir}/{filename}.csv', sep='\t')
        dt['type'] = pd.Categorical(dt['type'], categories=plots, ordered=True)

        fig = plt.figure()
        ax = plt.axes()

        # bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color)
        bar = sns.barplot(x="size", y="throughput", hue="type", data=dt, palette=point_color, ci='sd', errwidth=1.4, capsize=0.1)

        # Loop over the bars
        for bars, hatch in zip(ax.containers, hatches):
            # Set a different hatch for each group of bars
            for b in bars:
                b.set_hatch(hatch)
        # create the legend again to show the new hatching
        ax.legend()
        # plt.show()
        bar.set_xlabel('File size [KiB]')
        bar.set_ylabel('Throughput [Gbps]')

        plt.legend(loc='lower right')
        plt.tight_layout()
        output_name = f'figure-nginx-{core}core-{"server" if di == "tx" else "client"}'
        # plt.savefig(f'{current_dir}/{output_name}.png')
        plt.savefig(f'{current_dir}/{output_name}.svg')
