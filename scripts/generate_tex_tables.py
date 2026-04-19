import os
import re

def generate_performance_tex():
    # RL Agent Data
    agent_data = [
        ("QLearning", "select\\_action", "3.00 ns"),
        ("QLearning", "update", "134.86 ns"),
        ("SARSA", "select\\_action", "5.38 ns"),
        ("SARSA", "update", "136.60 ns"),
        ("DoubleQLearning", "select\\_action", "2.97 ns"),
        ("DoubleQLearning", "update", "248.15 ns"),
        ("ExpectedSARSA", "select\\_action", "3.00 ns"),
        ("ExpectedSARSA", "update", "149.96 ns"),
        ("REINFORCE", "select\\_action", "63.70 ns"),
        ("REINFORCE", "update", "194.03 ns"),
    ]

    # Algorithm Data
    algo_data = [
        ("XESReader", "read (Domestic)", "142.10 ms"),
        ("PetriNet", "is\\_structural\\_workflow\\_net", "840.00 ns"),
        ("TBR", "Standard Replayer", "6.50 $\\mu$s"),
        ("TBR", "BCINR Optimized Replayer", "4.24 $\\mu$s"),
    ]

    # Generate Agent Table
    tex = "\\begin{table}[ht]\n\\centering\n\\begin{tabular}{llr}\n\\toprule\n"
    tex += "Agent Class & Operation & Latency \\\\\n\\midrule\n"
    for agent, op, latency in agent_data:
        tex += f"{agent} & {op} & {latency} \\\\\n"
    tex += "\\bottomrule\n\\end{tabular}\n"
    tex += "\\caption{Reinforcement Learning Agent Micro-Benchmarks}\n"
    tex += "\\label{tab:agent_performance}\n\\end{table}\n\n"

    # Generate Algorithm Table
    tex += "\\begin{table}[ht]\n\\centering\n\\begin{tabular}{llr}\n\\toprule\n"
    tex += "Component & Operation & Performance \\\\\n\\midrule\n"
    for comp, op, perf in algo_data:
        tex += f"{comp} & {op} & {perf} \\\\\n"
    tex += "\\bottomrule\n\\end{tabular}\n"
    tex += "\\caption{Core Process Mining Algorithm Benchmarks}\n"
    tex += "\\label{tab:algo_performance}\n\\end{table}\n"

    with open("performance_results.tex", "w") as f:
        f.write(tex)
    print("Generated performance_results.tex with final optimized algorithm results")

if __name__ == "__main__":
    generate_performance_tex()
