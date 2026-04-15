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
    title: '后端与云原生主线',
    Svg: require('@site/static/img/undraw_docusaurus_mountain.svg').default,
    description: (
      <>
        围绕服务治理、容器编排、网络链路、配置管理、可观测性这些一线问题来写，不只讲概念。
      </>
    ),
  },
  {
    title: 'JVM / Linux 偏工程实践',
    Svg: require('@site/static/img/undraw_docusaurus_tree.svg').default,
    description: (
      <>
        重点放在性能、排障、线上稳定性和工程经验，尽量写成真正能拿来解决问题的文章。
      </>
    ),
  },
  {
    title: 'LLM 落地与系统设计',
    Svg: require('@site/static/img/undraw_docusaurus_react.svg').default,
    description: (
      <>
        不只聊模型概念，也会写 RAG、上下文工程、提示词编排、服务化部署和实际成本控制。
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
