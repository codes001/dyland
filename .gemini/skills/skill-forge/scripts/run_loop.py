#!/usr/bin/env python3
"""Optimize skill description for triggering accuracy."""

import argparse
import json
import os
import random
import sys
import time
from pathlib import Path


def load_eval_set(path: str) -> list[dict]:
    """Load trigger evaluation queries."""
    with open(path, "r") as f:
        data = json.load(f)
    if isinstance(data, dict) and "evals" in data:
        return data["evals"]
    return data


def split_eval_set(evals: list[dict], train_ratio: float = 0.6) -> tuple[list[dict], list[dict]]:
    """Split eval set into train and test."""
    shuffled = evals.copy()
    random.shuffle(shuffled)
    split_idx = int(len(shuffled) * train_ratio)
    return shuffled[:split_idx], shuffled[split_idx:]


def evaluate_description(description: str, eval_set: list[dict], model: str, runs_per_query: int = 3) -> dict:
    """
    Evaluate a description against an eval set.

    In a real implementation, this would call the model API to check
    whether the skill triggers for each query. Here we provide the
    framework and scoring logic.
    """
    results = []
    correct = 0
    total = 0

    for item in eval_set:
        query = item["query"]
        should_trigger = item["should_trigger"]

        # Placeholder: In production, this calls the model with the description
        # and checks if the skill is consulted. For now, we simulate.
        triggered = False  # Would be determined by actual model call

        # Simulate based on keyword overlap for demonstration
        desc_words = set(description.lower().split())
        query_words = set(query.lower().split())
        overlap = len(desc_words & query_words)
        triggered = overlap > 2  # Naive simulation

        match = triggered == should_trigger
        if match:
            correct += 1
        total += 1

        results.append({
            "query": query,
            "should_trigger": should_trigger,
            "triggered": triggered,
            "correct": match,
        })

    return {
        "accuracy": correct / total if total > 0 else 0.0,
        "correct": correct,
        "total": total,
        "results": results,
    }


def propose_improvement(current_description: str, train_results: dict, model: str) -> str:
    """
    Call model to propose an improved description based on failures.

    In production, this would make an actual API call. Here we provide
    the prompt framework.
    """
    failures = [r for r in train_results["results"] if not r["correct"]]

    if not failures:
        return current_description

    prompt = f"""You are optimizing a skill description for an AI assistant.

Current description:
{current_description}

The following queries were misclassified:
"""
    for f in failures:
        prompt += f"\n- Query: '{f['query']}'\n  Should trigger: {f['should_trigger']}\n  Actually triggered: {f['triggered']}"

    prompt += """\n\nPlease propose an improved description that:
1. Fixes the misclassifications above
2. Remains concise (under 100 words ideally)
3. Includes both what the skill does AND specific contexts for when to use it
4. Is slightly "pushy" to combat undertriggering — make sure to mention related concepts

Return ONLY the new description text, nothing else."""

    # Placeholder: In production, call model API here
    # For now, return current with a note
    return current_description + " (optimized)"


def run_optimization(eval_set: list[dict], skill_path: str, model: str, max_iterations: int, verbose: bool) -> dict:
    """Run the full optimization loop."""
    skill_md_path = os.path.join(skill_path, "SKILL.md")
    if not os.path.exists(skill_md_path):
        print(f"ERROR: SKILL.md not found at {skill_md_path}")
        sys.exit(1)

    # Extract current description
    with open(skill_md_path, "r") as f:
        content = f.read()

    # Simple frontmatter extraction
    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            import yaml
            try:
                fm = yaml.safe_load(parts[1]) or {}
                current_description = fm.get("description", "")
            except Exception:
                current_description = ""
        else:
            current_description = ""
    else:
        current_description = ""

    if not current_description:
        print("ERROR: Could not extract description from SKILL.md")
        sys.exit(1)

    train_set, test_set = split_eval_set(eval_set)
    print(f"Eval set split: {len(train_set)} train, {len(test_set)} test")

    best_description = current_description
    best_test_score = 0.0
    history = []

    for iteration in range(1, max_iterations + 1):
        print(f"\n=== Iteration {iteration}/{max_iterations} ===")

        # Evaluate on train
        if verbose:
            print("Evaluating on train set...")
        train_results = evaluate_description(current_description, train_set, model)
        print(f"Train accuracy: {train_results['accuracy']:.2%} ({train_results['correct']}/{train_results['total']})")

        # Evaluate on test
        if verbose:
            print("Evaluating on test set...")
        test_results = evaluate_description(current_description, test_set, model)
        print(f"Test accuracy: {test_results['accuracy']:.2%} ({test_results['correct']}/{test_results['total']})")

        history.append({
            "iteration": iteration,
            "description": current_description,
            "train_accuracy": train_results["accuracy"],
            "test_accuracy": test_results["accuracy"],
        })

        # Track best by test score
        if test_results["accuracy"] > best_test_score:
            best_test_score = test_results["accuracy"]
            best_description = current_description
            print(f"New best description (test score: {best_test_score:.2%})")

        # Stop if perfect
        if test_results["accuracy"] >= 1.0:
            print("Perfect test score achieved!")
            break

        # Propose improvement
        if iteration < max_iterations:
            if verbose:
                print("Proposing description improvement...")
            current_description = propose_improvement(current_description, train_results, model)
            time.sleep(0.5)  # Rate limiting placeholder

    return {
        "best_description": best_description,
        "best_test_score": best_test_score,
        "history": history,
    }


def main():
    parser = argparse.ArgumentParser(description="Optimize skill description for triggering")
    parser.add_argument("--eval-set", required=True, help="Path to trigger eval JSON")
    parser.add_argument("--skill-path", required=True, help="Path to skill directory")
    parser.add_argument("--model", required=True, help="Model ID for evaluation")
    parser.add_argument("--max-iterations", type=int, default=5, help="Max optimization iterations")
    parser.add_argument("--verbose", action="store_true", help="Verbose output")
    parser.add_argument("--output", help="Output path for results JSON")
    args = parser.parse_args()

    eval_set = load_eval_set(args.eval_set)
    print(f"Loaded {len(eval_set)} eval queries")

    result = run_optimization(
        eval_set,
        args.skill_path,
        args.model,
        args.max_iterations,
        args.verbose,
    )

    print(f"\n=== Optimization Complete ===")
    print(f"Best test score: {result['best_test_score']:.2%}")
    print(f"\nBest description:")
    print(result["best_description"])

    output_path = args.output or os.path.join(args.skill_path, "description_optimization.json")
    with open(output_path, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\nResults saved to: {output_path}")


if __name__ == "__main__":
    main()
