#!/usr/bin/env python3
"""
为中文数据集生成 keyword_index.json 文件
从 card.csv 转换，格式为 {question: [title]}
"""

import json
import csv
from pathlib import Path

def generate_keyword_index():
    csv_path = Path(__file__).parent / "card.csv"
    json_path = Path(__file__).parent / "keyword_index.json"
    
    if not csv_path.exists():
        print(f"❌ CSV 文件不存在: {csv_path}")
        return
    
    try:
        keyword_index = {}
        
        with open(csv_path, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            for row in reader:
                title = row.get('title', '').strip()
                question = row.get('question', '').strip()
                
                if title and question:
                    if question not in keyword_index:
                        keyword_index[question] = []
                    keyword_index[question].append(title)
        
        # 生成 JSON
        with open(json_path, 'w', encoding='utf-8') as f:
            json.dump(keyword_index, f, ensure_ascii=False, indent=2)
        
        print(f"✅ 已生成 {json_path}，包含 {len(keyword_index)} 个关键词")
        print(f"📋 示例:")
        for idx, (keyword, titles) in enumerate(sorted(keyword_index.items())[:5]):
            print(f"   '{keyword}' -> {titles}")
        
    except Exception as e:
        print(f"❌ 错误: {e}")

if __name__ == "__main__":
    generate_keyword_index()
