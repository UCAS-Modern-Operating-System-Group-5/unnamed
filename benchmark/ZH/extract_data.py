#!/usr/bin/env python3
"""
从 train.json 提取前100条数据：
1. 创建 txt 文件（文件名=title，内容=context）
2. 将 title 和 question 添加到 card.csv
"""

import json
import os
import csv
import re

def sanitize_filename(filename):
    """清理文件名中的非法字符"""
    # 移除或替换非法字符
    filename = re.sub(r'[<>:"/\\|?*]', '', filename)
    # 限制文件名长度
    if len(filename) > 200:
        filename = filename[:200]
    # 移除前后空格
    filename = filename.strip()
    # 如果文件名为空，使用默认名称
    if not filename:
        filename = "untitled"
    return filename

def main():
    # 读取 JSON 数据
    with open('docs/dataset/train.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # 创建输出目录
    output_dir = 'docs/extracted'
    os.makedirs(output_dir, exist_ok=True)
    
    # 准备 CSV 数据
    card_data = []
    
    # 提取前100条数据
    count = 0
    for item in data['data']:
        for paragraph in item['paragraphs']:
            if count >= 100:
                break
            
            title = paragraph.get('title', '').strip()
            context = paragraph.get('context', '').strip()
            
            # 获取第一个问题（如果有）
            question = ''
            if paragraph.get('qas') and len(paragraph['qas']) > 0:
                question = paragraph['qas'][0].get('question', '').strip()
            
            # 跳过没有 title 或 context 的数据
            if not title or not context:
                continue
            
            # 创建文件名
            safe_title = sanitize_filename(title)
            filename = f"{count+1:03d}_{safe_title}.txt"
            filepath = os.path.join(output_dir, filename)
            
            # 写入文本文件
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(context)
            
            # 添加到 card 数据
            card_data.append({
                'title': title,
                'question': question
            })
            
            count += 1
            print(f"已处理 {count}/100: {title[:50]}...")
        
        if count >= 100:
            break
    
    # 写入 card.csv
    with open('card.csv', 'w', encoding='utf-8', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=['title', 'question'])
        writer.writeheader()
        writer.writerows(card_data)
    
    print(f"\n完成！共提取 {count} 条数据")
    print(f"文本文件保存在: {output_dir}/")
    print(f"card.csv 已更新")

if __name__ == '__main__':
    main()
