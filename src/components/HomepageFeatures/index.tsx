import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<'svg'>>;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: '基础概念讲人话',
    Svg: require('@site/static/img/undraw_docusaurus_mountain.svg').default,
    description: (
      <>
        不靠术语轰炸，先把 Kubernetes 里真正关键的抽象讲顺，适合快速建立整体认知。
      </>
    ),
  },
  {
    title: '偏实战，不只贴 YAML',
    Svg: require('@site/static/img/undraw_docusaurus_tree.svg').default,
    description: (
      <>
        更关注为什么这么配、出了问题怎么查，而不是只给一段“能跑但看不懂”的配置。
      </>
    ),
  },
  {
    title: '可长期积累成知识库',
    Svg: require('@site/static/img/undraw_docusaurus_react.svg').default,
    description: (
      <>
        文档区适合整理体系化内容，博客区适合记录排障、复盘和临时想法，后面很容易越写越成型。
      </>
    ),
  },
];

function Feature({title, Svg, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <Svg className={styles.featureSvg} role="img" />
      </div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
