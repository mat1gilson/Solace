"""
Solace Protocol Python SDK Setup
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="solace-protocol-python",
    version="1.0.0",
    author="Solace Protocol Team",
    author_email="team@solaceprotocol.com",
    description="Python SDK for Solace Protocol autonomous agent commerce framework",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/solaceprotocol/solace-protocol",
    project_urls={
        "Bug Tracker": "https://github.com/solaceprotocol/solace-protocol/issues",
        "Documentation": "https://docs.solaceprotocol.com",
        "Source": "https://github.com/solaceprotocol/solace-protocol",
    },
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: System :: Distributed Computing",
        "Topic :: Office/Business :: Financial",
    ],
    packages=find_packages(),
    python_requires=">=3.9",
    install_requires=[
        "solana>=0.30.0",
        "solders>=0.18.0",
        "httpx>=0.25.0",
        "websockets>=11.0",
        "pydantic>=2.5.0",
        "cryptography>=41.0.0",
        "aiohttp>=3.9.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.4.0",
            "pytest-asyncio>=0.21.0",
            "black>=23.0.0",
            "flake8>=6.0.0",
            "mypy>=1.7.0",
            "sphinx>=7.1.0",
        ],
        "ml": [
            "numpy>=1.24.0",
            "pandas>=2.1.0",
            "scikit-learn>=1.3.0",
        ],
    },
    entry_points={
        "console_scripts": [
            "solace-agent=solace_protocol.cli:main",
        ],
    },
    keywords="solana blockchain autonomous agents ai commerce defi crypto",
    include_package_data=True,
    zip_safe=False,
) 