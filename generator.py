#!/usr/bin/env python3
"""
Generate random git commits with random dates, messages, and file operations.
"""

import os
import subprocess
import random
from datetime import datetime, timedelta

# Configuration
NUM_COMMITS = 5

# Commit message templates
COMMIT_MESSAGES = [
    "Fix bug in {component}",
    "Add {feature} functionality",
    "Update {component}",
    "Refactor {component} code",
    "Improve {feature} performance",
    "Remove deprecated {component}",
    "Add tests for {feature}",
    "Update documentation for {component}",
    "Fix typo in {component}",
    "Optimize {feature}",
    "Clean up {component}",
    "Enhance {feature}",
    "Fix issue with {component}",
    "Implement {feature}",
    "Resolve merge conflicts",
    "Update dependencies",
    "Add error handling",
    "Improve logging",
    "Fix security issue",
    "Add configuration options",
]

COMPONENTS = [
    "auth module", "database", "API", "UI components", "user service",
    "payment processing", "notification system", "cache layer", "middleware",
    "routing", "validation", "error handling", "configuration", "models"
]

FEATURES = [
    "user authentication", "data validation", "caching", "API endpoints",
    "search functionality", "filtering", "pagination", "export feature",
    "notifications", "analytics", "reporting", "integration"
]


def generate_random_date():
    """Generate a random date within the last year."""
    end_date = datetime.now()
    start_date = end_date - timedelta(days=365)
    random_days = random.randint(0, 365)
    random_seconds = random.randint(0, 86400)
    return start_date + timedelta(days=random_days, seconds=random_seconds)


def generate_commit_message():
    """Generate a random commit message."""
    template = random.choice(COMMIT_MESSAGES)
    if "{component}" in template:
        return template.format(component=random.choice(COMPONENTS))
    elif "{feature}" in template:
        return template.format(feature=random.choice(FEATURES))
    return template


def get_existing_files():
    """Get list of existing files in the repository."""
    files = []
    for root, _, filenames in os.walk('.'):
        for filename in filenames:
            if '.git' not in root:
                filepath = os.path.join(root, filename)
                files.append(filepath)
    return files


def create_file():
    """Create a new file with random content."""
    filename = f"file_{random.randint(1000, 9999)}.txt"
    content = f"Content generated at {datetime.now()}\n"
    content += f"Random data: {random.randint(1, 1000000)}\n"
    
    with open(filename, 'w') as f:
        f.write(content)
    
    return filename


def modify_file(filename):
    """Modify an existing file."""
    with open(filename, 'a') as f:
        f.write(f"\nModified at {datetime.now()}\n")
        f.write(f"Additional data: {random.randint(1, 1000000)}\n")
    
    return filename


def delete_file(filename):
    """Delete a file."""
    os.remove(filename)
    return filename


def perform_random_operation():
    """Perform a random file operation (create, modify, or delete)."""
    existing_files = get_existing_files()
    
    # If no files exist, create one
    if not existing_files:
        filename = create_file()
        operation = "create"
    else:
        # Random operation with weighted probabilities
        # 40% create, 40% modify, 20% delete
        rand = random.random()
        if rand < 0.4:
            filename = create_file()
            operation = "create"
        elif rand < 0.8:
            filename = random.choice(existing_files)
            modify_file(filename)
            operation = "modify"
        else:
            # Only delete if we have more than 5 files
            if len(existing_files) > 5:
                filename = random.choice(existing_files)
                delete_file(filename)
                operation = "delete"
            else:
                filename = random.choice(existing_files)
                modify_file(filename)
                operation = "modify"
    
    return filename, operation


def create_commit(commit_date):
    """Create a git commit with a specific date."""
    filename, operation = perform_random_operation()
    message = generate_commit_message()
    
    # Stage the file
    if operation == "delete":
        subprocess.run(["git", "rm", filename], check=True, 
                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    else:
        subprocess.run(["git", "add", filename], check=True,
                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    
    # Create commit with custom date
    date_str = commit_date.strftime("%Y-%m-%d %H:%M:%S")
    env = os.environ.copy()
    env["GIT_AUTHOR_DATE"] = date_str
    env["GIT_COMMITTER_DATE"] = date_str
    
    subprocess.run(
        ["git", "commit", "-m", message],
        env=env,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )
    
    return message, operation, filename


def main():
    """Main function to generate commits."""
    print("Git Commit Generator")
    print("=" * 50)

    # Generate random dates and sort them
    dates = [generate_random_date() for _ in range(NUM_COMMITS)]
    dates.sort()
    
    print(f"\nGenerating {NUM_COMMITS} commits...")
    
    for i, date in enumerate(dates, 1):
        try:
            message, operation, filename = create_commit(date)
            if i % 50 == 0:
                print(f"Progress: {i}/{NUM_COMMITS} commits created")
        except Exception as e:
            print(f"Error creating commit {i}: {e}")
            continue
    
    print(f"\n✓ Successfully generated {NUM_COMMITS} commits!")
    print(f"Repository location: {os.path.abspath('.')}")
    print("\nYou can view the commits with: git log --oneline")


if __name__ == "__main__":
    main()

Modified at 2025-09-30 17:02:14.456141
Additional data: 342709
